use url::Url;

pub fn parse_target_url(input: &str) -> Option<Url> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        Url::parse(trimmed).ok()
    } else {
        Url::parse(&format!("https://{trimmed}")).ok()
    }
}
