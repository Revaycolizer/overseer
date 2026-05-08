use serde_json::Value;

pub fn repair_json_literals(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_quotes = false;
    let mut escaped = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
        match c {
            '"' if !escaped => {
                in_quotes = !in_quotes;
                out.push(c);
            }
            '\\' if !escaped => {
                escaped = true;
                out.push(c);
            }
            '\n' | '\r' if in_quotes => {
                out.push_str("\\n");
                escaped = false;
            }
            '\t' if in_quotes => {
                out.push_str("\\t");
                escaped = false;
            }
            _ => {
                out.push(c);
                escaped = false;
            }
        }
        i += 1;
    }
    out
}
