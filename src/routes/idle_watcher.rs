use crate::Route;
use dioxus::prelude::*;
use crate::state::GlobalState;
use crate::engine::utils::system_idle::get_system_idle_secs;
fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }
#[component]
pub fn IdleWatcher() -> Element {
    let nav = use_navigator();
    let route = use_route::<Route>();
    let mut state = use_context::<Signal<GlobalState>>();
    let mut entered_idle_at = use_signal(|| 0u64);
    let mut idle_time = use_signal(|| 0u64);
    let route_for_future = route.clone();
    use_future(move || {
        crate::engine::voice::wake_word::start_wake_word_listener();
        let nav = nav.clone();
        let route = route_for_future.clone();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if *state.peek().is_inhibited.peek() { continue; }
                if crate::engine::voice::wake_word::try_recv_wake() {
                    state.write().is_listening.set(true);
                    state.write().is_inhibited.set(true);
                    state.write().voice_transcript.set(None);
                    state.write().voice_result.set(None);
                    nav.push(Route::ComputerControl {});
                    crate::engine::voice::tts::speak_locally("How can I help you?");
                    continue;
                }
                let idle_secs = get_system_idle_secs();
                idle_time.set(idle_secs);
                let entered = *entered_idle_at.peek();
                if idle_secs >= 30 && entered == 0 {
                    entered_idle_at.set(now());
                    nav.push(Route::SystemIdleSecureLockScreen {});
                } else if idle_secs < 30 && entered != 0 {
                    entered_idle_at.set(0);
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        if now() - entered > 2 { nav.push(Route::Conversations {}); }
                    }
                }
            }
        }
    });
    macro_rules! reset_idle {
        () => {{
            let route = route.clone();
            move |_| {
                if let Route::SystemIdleSecureLockScreen {} = route {
                    let start = *entered_idle_at.peek();
                    if start != 0 && now() - start > 2 { nav.push(Route::Conversations {}); }
                }
            }
        }};
    }
    rsx! {
        div {
            class: "h-screen w-screen relative overflow-hidden outline-none",
            tabindex: "0", autofocus: "true",
            onmousedown: reset_idle!(),
            onkeydown: reset_idle!(),
            onmousemove: reset_idle!(),
            div { class: "fixed bottom-0 right-0 p-1 text-[8px] opacity-10 pointer-events-none z-[9999]",
                {
                    let cur_idle = idle_time();
                    let cur_start = entered_idle_at();
                    let grace = if cur_start != 0 { now().saturating_sub(cur_start) } else { 0 };
                    format!("IDLE: {}s | GRACE: {}s", cur_idle, grace)
                }
            }
            Outlet::<Route> {}
        }
    }
}