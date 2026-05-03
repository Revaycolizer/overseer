pub fn repair_json_literals(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_quotes = false;
    let mut escaped = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
        if !in_quotes && c == '[' && i + 1 < chars.len() && chars[i+1].is_alphabetic() {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == ' ') { j += 1; }
            if j < chars.len() && chars[j] == ']' {
                out.push('"');
                out.push_str(&chars[i..=j].iter().collect::<String>());
                out.push('"');
                i = j + 1; continue;
            }
        }
        match c {
            '"' if !escaped => { in_quotes = !in_quotes; out.push(c); }
            '\\' if !escaped => { escaped = true; out.push(c); }
            '\n' | '\r' if in_quotes => { out.push_str("\\n"); escaped = false; }
            '\t' if in_quotes => { out.push_str("\\t"); escaped = false; }
            _ => { out.push(c); escaped = false; }
        }
        i += 1;
    }
    out
}
