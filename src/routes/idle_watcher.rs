use crate::Route;
use dioxus::prelude::*;
use crate::state::GlobalState;

#[component]
pub fn IdleWatcher() -> Element {
    let nav = use_navigator();
    let route = use_route::<Route>();
    let state = use_context::<Signal<GlobalState>>();

    let mut last_mouse_pos = use_signal(|| (0, 0));
    let mut last_keys = use_signal(|| Vec::<device_query::Keycode>::new());
    let mut entered_idle_at = use_signal(|| 0u64);

    // Global Activity Monitor (Device-Wide)
    use_future(move || async move {
        println!("📍 Attempting to start global device monitor...");
        let device_state = match std::panic::catch_unwind(|| device_query::DeviceState::new()) {
            Ok(ds) => {
                println!("✅ Global device monitor initialized!");
                ds
            },
            Err(_) => {
                eprintln!("❌ Global device monitor failed to initialize (X11/Permissions issue?)");
                return;
            },
        };
        
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            
            use device_query::DeviceQuery;
            let mouse = device_state.get_mouse();
            let keys = device_state.get_keys();
            
            let current_mouse = mouse.coords;
            let last_mouse = *last_mouse_pos.peek();
            
            // Movement threshold (ignore jitter)
            let mouse_moved = (current_mouse.0 - last_mouse.0).abs() > 2 || 
                             (current_mouse.1 - last_mouse.1).abs() > 2;
                             
            let mouse_clicked = mouse.button_pressed.iter().any(|&b| b);
            
            // Key change detection (more reliable than presence)
            let current_keys = keys.clone();
            let keys_changed = current_keys != *last_keys.peek();
            
            if mouse_moved || mouse_clicked || keys_changed {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                let mut last_activity = state.peek().last_activity;
                last_activity.set(now);
                
                if mouse_moved { last_mouse_pos.set(current_mouse); }
                if keys_changed { last_keys.set(current_keys); }

                // Auto-Wake from Idle state
                let idle_start = *entered_idle_at.peek();
                if idle_start != 0 && now - idle_start > 2 {
                    println!("🌅 Auto-Wake triggered by global activity!");
                    nav.push(Route::Conversations {});
                }
            }
        }
    });

    let mut idle_time = use_signal(|| 0u64);

    // Idle Timer Loop
    use_future(move || {
        let tn = nav.clone();
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                
                let (last, inhibited) = {
                    let s = state.peek();
                    let last_val = *s.last_activity.peek();
                    let inhib_val = *s.is_inhibited.peek();
                    (last_val, inhib_val)
                };
                
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                idle_time.set(now - last);

                // For production, this should be 300 (5 mins). 30s is a good compromise for testing.
                if !inhibited && (now - last) >= 30 { 
                    println!("🌙 System IDLE! Navigating to lock screen... (Idle Time: {}s)", now - last);
                    tn.push(Route::SystemIdleSecureLockScreen {});
                }
            }
        }
    });


    // Sync idle entry time
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    if let Route::SystemIdleSecureLockScreen {} = route {
        if *entered_idle_at.peek() == 0 {
            entered_idle_at.set(now);
        }
    } else if *entered_idle_at.peek() != 0 {
        entered_idle_at.set(0);
    }

    rsx! {
        div { 
            class: "h-screen w-screen relative overflow-hidden outline-none",
            tabindex: "0",
            autofocus: "true",
            onmousedown: {
                let route = route.clone();
                let nav = nav.clone();
                let state = state.clone();
                move |_| {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let mut last_activity = state.peek().last_activity;
                    last_activity.set(now);
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        if now - *entered_idle_at.peek() > 2 {
                            nav.push(Route::Conversations {});
                        }
                    }
                }
            },
            onkeydown: {
                let route = route.clone();
                let nav = nav.clone();
                let state = state.clone();
                move |_| {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let mut last_activity = state.peek().last_activity;
                    last_activity.set(now);
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        if now - *entered_idle_at.peek() > 2 {
                            nav.push(Route::Conversations {});
                        }
                    }
                }
            },
            onmousemove: {
                let route = route.clone();
                let nav = nav.clone();
                let state = state.clone();
                move |_| {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                    let mut last_activity = state.peek().last_activity;
                    last_activity.set(now);
                    if let Route::SystemIdleSecureLockScreen {} = route {
                        if now - *entered_idle_at.peek() > 2 {
                            nav.push(Route::Conversations {});
                        }
                    }
                }
            },

            // Hidden Debug Timer (visible only in dev)
            div { class: "fixed bottom-0 right-0 p-1 text-[8px] opacity-10 pointer-events-none z-[9999]",
                "IDLE: {idle_time}s | GRACE: {std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() - *entered_idle_at.peek()}s"
            }
            
            Outlet::<Route> {}
        }
    }
}
