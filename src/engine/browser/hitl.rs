pub fn detect_hitl_reason(url: &str, title: &str, body: &str) -> Option<&'static str> {
    let lower = format!("{} {}", title, body).to_lowercase();
    if lower.contains("captcha")
        || lower.contains("hcaptcha")
        || lower.contains("recaptcha")
        || lower.contains("verify you're human")
    {
        return Some("captcha");
    }
    if lower.contains("login")
        || lower.contains("sign in")
        || lower.contains("password")
        || url.contains("login")
        || url.contains("signin")
    {
        return Some("login");
    }
    if lower.contains("verification code")
        || lower.contains("two-factor")
        || lower.contains("2fa")
        || lower.contains("otp")
    {
        return Some("2fa");
    }
    if lower.contains("card number") || lower.contains("credit card") || lower.contains("payment") {
        return Some("payment");
    }
    if lower.contains("404")
        || lower.contains("not found")
        || lower.contains("error") && lower.contains("page")
    {
        return Some("error_page");
    }
    None
}
