# Rust 运行沙箱镜像。
# 与 src/api/code_runner/languages.rs 的 rust 注册项对齐：
#   image: yggdrasil-runner-rust:latest
#   run_cmd: /usr/local/bin/run-rust.sh
#
# rustc 编译 + 运行是两步，但 docker.rs 的注入脚本用 exec 执行 run_cmd，
# exec 会替换 shell 进程，导致 "A && B" 的后半段不执行。
# 因此镜像内置 run-rust.sh wrapper 封装「编译并运行」逻辑。
FROM yggdrasil-runner-base:latest

RUN apk add --no-cache rust

COPY run-rust.sh /usr/local/bin/run-rust.sh
RUN chmod +x /usr/local/bin/run-rust.sh

COPY runner.toml /runner.toml

USER runner
