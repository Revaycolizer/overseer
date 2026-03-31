use dioxus::prelude::document::eval;

pub fn copy_to_clipboard(text: String) {
    let escaped = text.replace('\\', "\\\\").replace('\'', "\\'").replace('"', "\\\"").replace('\n', "\\n");
    let eval_str = format!("navigator.clipboard.writeText('{}')", escaped);
    let _ = eval(&eval_str);
}
