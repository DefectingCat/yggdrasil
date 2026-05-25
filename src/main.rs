use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);
    let mut text = use_signal(|| "...".to_string());

    rsx! {
        div { style: "padding: 2rem; font-family: system-ui, sans-serif;",
            h1 { "Dioxus SSR Fullstack" }

            p { "This page is rendered on the server and hydrated on the client." }

            div { style: "margin: 1rem 0;",
                h2 { "Counter: {count}" }
                button { onclick: move |_| count += 1, "Increment" }
                button { onclick: move |_| count -= 1, "Decrement" }
            }

            div { style: "margin: 1rem 0;",
                h2 { "Server Function" }
                button {
                    onclick: move |_| async move {
                        match get_server_greeting().await {
                            Ok(data) => text.set(data),
                            Err(e) => text.set(format!("Error: {}", e)),
                        }
                    },
                    "Call Server"
                }
                p { "Server said: {text}" }
            }
        }
    }
}

#[server]
async fn get_server_greeting() -> Result<String, ServerFnError> {
    Ok("Hello from the server!".to_string())
}
