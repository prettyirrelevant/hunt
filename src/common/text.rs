use std::sync::LazyLock;

use regex::Regex;

/// Boards serve HTML, sometimes entity-escaped twice.
pub fn markdown(html: &str) -> String {
    let unescaped = html_escape::decode_html_entities(html);
    let md = htmd::convert(&unescaped).unwrap_or_else(|_| unescaped.into_owned());
    md.trim().to_string()
}

static PII: LazyLock<[(Regex, &str); 4]> = LazyLock::new(|| {
    let re = |p: &str| Regex::new(p).expect("valid pattern");
    [
        (re(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b"), "[email]"),
        (re(r"(?i)\b(sk|pk|ghp|gho|xox[abp]|AKIA)[-_a-z0-9]{12,}\b"), "[secret]"),
        (re(r"\b[a-fA-F0-9]{32,}\b"), "[secret]"),
        (re(r"(?i)https?://[^\s)]*[?&](token|key|secret|sig|signature|password)=[^\s)]*[^\s).,;]"), "[private link]"),
    ]
});

pub fn scrub(text: &str) -> String {
    let text =
        PII.iter().fold(text.to_string(), |text, (pattern, mask)| pattern.replace_all(&text, *mask).into_owned());
    // Only ten or more digits count as a phone number. Date ranges have fewer.
    PHONE
        .replace_all(&text, |m: &regex::Captures| {
            let digits = m[0].chars().filter(char::is_ascii_digit).count();
            if digits >= 10 { "[phone]".to_string() } else { m[0].to_string() }
        })
        .into_owned()
}

static PHONE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\+?\d[\d\s().-]{8,}\d").expect("valid pattern"));
