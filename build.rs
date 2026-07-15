//! 构建脚本:采集 git / rustc / 编译时刻信息,通过 cargo:rustc-env 注入,
//! 供 src/build_info.rs 在编译期内联读取(env! 宏,零运行时开销)。
//!
//! 设计取舍见 src/build_info.rs 顶部注释。要点:
//! - 只用 std,不引入 build-dependencies(项目所有 server 依赖都 optional,
//!   为启动 banner 拉 vergen/git2 不划算)。
//! - git 不可用(非仓库 / tarball 构建)时降级为 "unknown",不 fail the build。
//! - 声明 rerun-if-changed=.git/HEAD,否则 cargo 默认仅在 build.rs 自身变化时
//!   重跑,会导致打印的还是旧 hash。

use std::process::Command;

fn main() {
    // 切换提交 / 分支后重新采集(读取 .git/HEAD 指向的新 ref)。
    println!("cargo:rerun-if-changed=.git/HEAD");
    // 工作区脏状态变化时 describe 的 --dirty 后缀也会变,重跑一次。
    println!("cargo:rerun-if-changed=.git/index");

    set_env("YGG_BUILD_GIT_DESCRIBE", git_describe());
    set_env("YGG_BUILD_GIT_HASH", git_hash());
    set_env("YGG_BUILD_GIT_COMMIT_DATE", git_commit_date());
    set_env("YGG_BUILD_RUSTC_VERSION", rustc_version());
    // 编译时刻(Unix 秒)。std 无 ISO 8601 格式化,存秒数由运行时 chrono 解析。
    set_env("YGG_BUILD_TIME", build_time_unix());
}

/// 注入一个编译期环境变量。
fn set_env(key: &str, value: String) {
    println!("cargo:rustc-env={key}={value}");
}

/// `git describe --tags --always --dirty`:最紧凑的"版本 + 提交数 + hash + 脏标记"。
fn git_describe() -> String {
    git_output(&["describe", "--tags", "--always", "--dirty"]).unwrap_or_else(|| "unknown".into())
}

/// 完整 40 位 commit hash。
fn git_hash() -> String {
    git_output(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into())
}

/// 提交时间(ISO 8601 strict,带时区偏移,可直接被 chrono 解析)。
fn git_commit_date() -> String {
    git_output(&["log", "-1", "--format=%cd", "--date=iso-strict"])
        .unwrap_or_else(|| "unknown".into())
}

/// 执行一条 git 命令,返回 trim 后的 stdout。失败返回 None(降级路径)。
fn git_output(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout)
        .ok()?
        .trim()
        .to_string()
        .into()
}

/// `rustc --version`,采集编译工具链。
fn rustc_version() -> String {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// 编译时刻(Unix 秒)。
fn build_time_unix() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}
