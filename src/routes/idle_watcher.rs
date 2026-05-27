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
    let mut is_listening_sig = state.peek().is_listening;
    let mut is_inhibited_sig = state.peek().is_inhibited;
    let mut voice_transcript_sig = state.peek().voice_transcript;
    let mut voice_result_sig = state.peek().voice_result;
    let route_for_future = route.clone();
    use_future(move || {
        crate::engine::voice::wake_word::start_wake_word_listener();
        let nav = nav.clone();
        let route = route_for_future.clone();
        async move {
            let mut tick = 0;
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if crate::engine::voice::wake_word::try_recv_panic() {
                    entered_idle_at.set(0);
                    is_listening_sig.set(false);
                    is_inhibited_sig.set(false);
                    crate::engine::voice::tts::speak_locally("Emergency reset. Exiting lock screen.");
                    nav.push(Route::Conversations {});
                    continue;
                }

                if crate::engine::voice::wake_word::try_recv_wake() {
                    crate::engine::voice::earcons::play_listening_chime();
                    is_listening_sig.set(true);
                    is_inhibited_sig.set(true);
                    voice_transcript_sig.set(None);
                    voice_result_sig.set(None);
                    nav.push(Route::ComputerControl {});
                    crate::engine::voice::tts::speak_locally("How can I help you?");
                    continue;
                }
                
                tick += 1;
                if tick >= 10 {
                    tick = 0;
                    if *is_inhibited_sig.peek() { continue; }
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