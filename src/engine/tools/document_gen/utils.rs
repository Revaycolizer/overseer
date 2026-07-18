
pub fn strip_grounding_markers(text: &str) -> String {
    let re_e = regex::Regex::new(r"\(?\[E[0-9]+\]\)?\s*").unwrap();
    let re_gid = regex::Regex::new(r"\[Grounded ID: [0-9]+\]\s*").unwrap();
    let cleaned = re_e.replace_all(text, "");
    re_gid.replace_all(&cleaned, "").to_string().replace("  ", " ").trim().to_string()
}


pub fn find_system_font() -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>, String> {
    let mut paths = vec!["assets/fonts/Roboto-Regular.ttf".to_string()];
    
    if cfg!(target_os = "windows") {
        paths.push("C:\\Windows\\Fonts\\arial.ttf".to_string());
    } else if cfg!(target_os = "macos") {
        paths.push("/Library/Fonts/Arial.ttf".to_string());
    } else {
        paths.push("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string());
        paths.push("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf".to_string());
    };

    // Generic Linux fallback: search for ANY ttf font in common dirs
    if cfg!(target_os = "linux") {
        let common_dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];
        for dir in common_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().map_or(false, |ext| ext == "ttf") {
                        paths.push(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    for path_str in paths {
        let path = std::path::Path::new(&path_str);
        if path.exists() {
            if let (Some(dir), Some(stem_os)) = (path.parent(), path.file_stem()) {
                let stem = stem_os.to_str().unwrap_or("");
                
                // Try 1: Load with the full stem (e.g. DejaVuSans)
                if let Ok(f) = genpdf::fonts::from_files(dir, stem, None) {
                    return Ok(f);
                }
                
                // Try 2: If stem is "Roboto-Regular", try loading as "Roboto" family
                if stem.ends_with("-Regular") {
                    let family = &stem[..stem.len() - 8];
                    if let Ok(f) = genpdf::fonts::from_files(dir, family, None) {
                        return Ok(f);
                    }
                }

                // Try 3: Special case for Roboto specifically
                if stem.contains("Roboto") {
                    if let Ok(f) = genpdf::fonts::from_files(dir, "Roboto", None) {
                        return Ok(f);
                    }
                }
            }
        }
    }
    Err("No system font found. Please install DejaVu or Liberation fonts.".into())
}
