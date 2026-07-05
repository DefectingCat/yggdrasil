//! 管理后台「代码试运行」页面。
//!
//! 作者在写作时可在此沙箱快速试运行代码（验证围栏 ` ```lang runnable ` 的预期输出），
//! 而无需进入文章渲染后才能运行。沙箱使用与读者相同的 StartExec / GetExecResult
//! 接口，受同一套资源钳制约束（admin 跳过速率限制，见 `start_exec`）。
//!
//! 仅 WASM 前端交互；语言在受支持集合内切换。

use dioxus::prelude::*;

use crate::components::code_runner::CodeRunner;
use crate::components::ui::ADMIN_CARD_CLASS;
use crate::infra::runner_config::ResourceLimits;

/// 受支持的语言集合（与 LANGUAGES 注册表 / CODE_RUNNER_LANGUAGES 对齐）。
const SUPPORTED_LANGS: &[&str] = &["python", "node"];

/// 默认示例源码（按语言）。
fn default_source(lang: &str) -> String {
    match lang {
        "python" => "print('Hello from author sandbox')\nfor i in range(3):\n    print(f'line {i}')\n".to_string(),
        "node" => "console.log('Hello from author sandbox');\n[0,1,2].forEach(i => console.log(`line ${i}`));\n".to_string(),
        _ => String::new(),
    }
}

/// 管理后台代码试运行页面。
#[component]
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
pub fn Runner() -> Element {
    let mut lang = use_signal(|| "python".to_string());
    // 语言切换时刷新示例源码（首次进入也有默认值）。
    let mut source = use_signal(|| default_source("python"));
    let mut overrides_json = use_signal(String::new);

    // overrides 解析用 use_memo 承载：render 体只读不写（Dioxus render purity），
    // 避免 render 期间 .set() override_error。畸形 JSON 标记在 memo 返回值里。
    let parsed = use_memo(move || {
        let raw = overrides_json();
        match serde_json::from_str::<ResourceLimits>(raw.trim()) {
            Ok(o) => (Some(o), String::new()),
            Err(_) => {
                if raw.trim().is_empty() {
                    (None, String::new())
                } else {
                    (None, "overrides JSON 格式错误，已忽略".to_string())
                }
            }
        }
    });
    let (overrides, override_error) = (parsed.read().0.clone(), parsed.read().1.clone());

    rsx! {
        div { class: "flex flex-col gap-6 max-w-4xl mx-auto w-full",
            div { class: "flex flex-col gap-2",
                h1 { class: "text-3xl font-extrabold tracking-tight", "代码试运行沙箱" }
                p { class: "text-base text-base-content/60",
                    "在此快速试运行代码，验证文章中可运行代码块的预期输出。资源钳制与读者侧一致，速率限制对 admin 放行。"
                }
            }

            div { class: "{ADMIN_CARD_CLASS} flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    label { class: "text-sm font-semibold text-base-content/70", "语言" }
                    div { class: "join",
                        for l in SUPPORTED_LANGS {
                            button {
                                key: "{l}",
                                class: (if lang() == *l { "btn btn-sm join-item btn-primary" } else { "btn btn-sm join-item btn-ghost" }).to_string(),
                                onclick: {
                                    let ll = (*l).to_string();
                                    move |_| {
                                        if ll != lang() {
                                            lang.set(ll.clone());
                                            source.set(default_source(&ll));
                                        }
                                    }
                                },
                                "{l}"
                            }
                        }
                    }
                }
                div { class: "flex flex-col gap-1",
                    label { class: "text-sm font-semibold text-base-content/70",
                        "资源覆盖 (JSON, 可选)"
                    }
                    input {
                        class: "input input-bordered input-sm font-mono w-full",
                        r#type: "text",
                        placeholder: "如 {{\"timeout_secs\":10,\"memory_mb\":512}}",
                        value: "{overrides_json()}",
                        oninput: move |e| overrides_json.set(e.value()),
                    }
                    if !override_error.is_empty() {
                        span { class: "text-xs text-error", "{override_error}" }
                    }
                }
            }

            CodeRunner {
                source: source(),
                language: lang(),
                overrides: overrides,
            }
        }
    }
}
