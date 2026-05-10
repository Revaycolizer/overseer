use crate::Route;
use dioxus::prelude::*;
use crate::state::GlobalState;
use crate::engine::utils::system_idle::get_system_idle_secs;

#[component]
pub fn IdleWatcher() -> Element {
    let nav = use_navigator();
    let route = use_route::<Route>();
    let state = use_context::<Signal<GlobalState>>();

    let mut entered_idle_at = use_signal(|| 0u64);
    let mut idle_time = use_signal(|| 0u64);

    use_future(move || {
        crate::engine::voice::wake_word::start_wake_word_listener();
        let nav = nav.clone();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if *state.peek().is_inhibited.peek() { continue; }

                let idle_secs = get_system_idle_secs();
                idle_time.set(idle_secs);

                if idle_secs >= 30 {
                    if *entered_idle_at.peek() == 0 {
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        entered_idle_at.set(now);
                        nav.push(Route::SystemIdleSecureLockScreen {});
                    }
                } else if *entered_idle_at.peek() != 0 {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    if now - *entered_idle_at.peek() > 2 {
                        entered_idle_at.set(0);
                        nav.push(Route::Conversations {});
                    }
                }
            }
        }
    });

    rsx! {
        div {
            class: "h-screen w-screen relative overflow-hidden outline-none",
            tabindex: "0", autofocus: "true",
            onmousedown: {
                let route = route.clone();
                move |_| {
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        let start = *entered_idle_at.peek();
                        if start != 0 {
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                            if now - start > 2 { nav.push(Route::Conversations {}); }
                        }
                    }
                }
            },
            onkeydown: {
                let route = route.clone();
                move |_| {
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        let start = *entered_idle_at.peek();
                        if start != 0 {
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                            if now - start > 2 { nav.push(Route::Conversations {}); }
                        }
                    }
                }
            },
            onmousemove: {
                let route = route.clone();
                move |_| {
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        let start = *entered_idle_at.peek();
                        if start != 0 {
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                            if now - start > 2 { nav.push(Route::Conversations {}); }
                        }
                    }
                }
            },

            div { class: "fixed bottom-0 right-0 p-1 text-[8px] opacity-10 pointer-events-none z-[9999]",
                {
                    let cur_idle = idle_time();
                    let cur_start = entered_idle_at();
                    let grace = if cur_start != 0 {
                        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().saturating_sub(cur_start)
                    } else {
                        0
                    };
                    format!("IDLE: {}s | GRACE: {}s", cur_idle, grace)
                }
            }
            Outlet::<Route> {}
        }
    }
}