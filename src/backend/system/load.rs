use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLoad {
    pub label: String,
    pub usage: u32,
}

pub async fn get_system_load() -> SystemLoad {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // 1. Try GPU Load (NVIDIA)
    let mut gpu_cmd = Command::new("nvidia-smi");
    gpu_cmd.args([
        "--query-gpu=utilization.gpu",
        "--format=csv,noheader,nounits",
    ]);
    #[cfg(windows)]
    gpu_cmd.creation_flags(CREATE_NO_WINDOW);
    let gpu_output = gpu_cmd.output();

    if let Ok(output) = gpu_output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Ok(usage) = s.parse::<u32>() {
                return SystemLoad {
                    label: "GPU Load".to_string(),
                    usage,
                };
            }
        }
    }

    // 2. Fallback to CPU Load
    // We use top to get a quick snapshot of CPU idle time on Unix-like systems.
    #[cfg(not(windows))]
    let cpu_output = Command::new("sh")
        .arg("-c")
        .arg("top -bn1 | grep '%Cpu(s)'")
        .output();

    #[cfg(not(windows))]
    if let Ok(output) = cpu_output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            // Example: %Cpu(s):  8.3 us,  1.5 sy,  0.0 ni, 88.6 id, ...
            if let Some(idle_part) = s.split(',').find(|part| part.contains("id")) {
                let idle_val = idle_part
                    .trim()
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<f32>().ok());

                if let Some(idle) = idle_val {
                    let usage = (100.0 - idle).round() as u32;
                    return SystemLoad {
                        label: "CPU Load".to_string(),
                        usage: usage.min(100),
                    };
                }
            }
        }
    }

    #[cfg(windows)]
    return SystemLoad {
        label: "System Load".to_string(),
        usage: 0,
    };

    // 3. Absolute fallback
    SystemLoad {
        label: "System Load".to_string(),
        usage: 0,
    }
}
