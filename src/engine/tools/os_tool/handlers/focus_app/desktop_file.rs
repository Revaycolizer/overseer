use std::path::PathBuf;

pub async fn find_linux_desktop_file(app_name: &str) -> Option<String> {
    if cfg!(target_os = "macos") {
        return None;
    }

    let search_paths = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".local/share/applications")
            } else {
                PathBuf::from("/tmp")
            }
        },
    ];

    let lower_name = app_name.to_lowercase();

    for path in search_paths {
        if let Ok(mut entries) = tokio::fs::read_dir(&path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                if file_name.ends_with(".desktop") {
                    if file_name.contains(&lower_name) {
                        return Some(entry.file_name().to_string_lossy().to_string());
                    }
                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                        for line in content.lines() {
                            if line.to_lowercase().starts_with("name=") {
                                let name_val = line[5..].trim().to_lowercase();
                                if name_val == lower_name {
                                    return Some(entry.file_name().to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
