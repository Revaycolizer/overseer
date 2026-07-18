use regex::Regex;

pub fn clean_text_content(raw: &str) -> String {
    let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("valid script regex");
    let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("valid style regex");
    let comment_re = Regex::new(r"(?is)<!--.*?-->").expect("valid comment regex");
    let tag_re = Regex::new(r"(?is)<[^>]+>").expect("valid tag regex");
    let whitespace_re = Regex::new(r"\s+").expect("valid whitespace regex");

    let without_scripts = script_re.replace_all(raw, " ");
    let without_styles = style_re.replace_all(&without_scripts, " ");
    let without_comments = comment_re.replace_all(&without_styles, " ");
    let without_tags = tag_re.replace_all(&without_comments, " ");
    let decoded = decode_html_entities(&without_tags);

    whitespace_re.replace_all(decoded.trim(), " ").to_string()
}

pub fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

pub fn truncate_for_tool(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

pub fn extract_html_title(html: &str) -> Option<String> {
    let title_re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").ok()?;
    title_re
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| clean_text_content(value.as_str()))
        .filter(|value| !value.is_empty())
}
