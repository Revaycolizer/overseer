pub fn strip_grounding_markers(text: &str) -> String {
    let re_e = regex::Regex::new(r"\(?\[E[0-9]+\]\)?\s*").unwrap();
    let re_gid = regex::Regex::new(r"\[Grounded ID: [0-9]+\]\s*").unwrap();
    let cleaned = re_e.replace_all(text, "");
    re_gid
        .replace_all(&cleaned, "")
        .to_string()
        .replace("  ", " ")
        .trim()
        .to_string()
}

/// Load a single TrueType font; no bold/italic sibling files are required.
pub fn find_system_font() -> Result<Vec<u8>, String> {
    let paths = [
        "assets/fonts/Roboto-Regular.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            if skrifa::FontRef::new(&bytes).is_ok() {
                return Ok(bytes);
            }
        }
    }
    Err("No TrueType font found. Install Arial, DejaVu Sans or Liberation Sans.".into())
}
