use hunt::common::text::markdown;

#[test]
fn escaped_html_becomes_markdown() {
    assert_eq!(markdown("&lt;p&gt;Build &lt;strong&gt;fast&lt;/strong&gt; things&lt;/p&gt;"), "Build **fast** things");
}

#[test]
fn contact_details_and_secrets_are_scrubbed() {
    let text =
        "Mail ada@example.com or call +234 803 123 4567. Key ghp_abcdefghijklmnop1234 in https://x.io/a?token=abc.";
    assert_eq!(hunt::common::text::scrub(text), "Mail [email] or call [phone]. Key [secret] in [private link].");
}

#[test]
fn engineering_numbers_survive_the_scrub() {
    let text = "Cut p99 latency from 180 ms to 40 ms across 12 services in 2024.";
    assert_eq!(hunt::common::text::scrub(text), text);
}

#[test]
fn date_ranges_are_not_phone_numbers() {
    assert_eq!(hunt::common::text::scrub("Backend lead, 2019 - 2023"), "Backend lead, 2019 - 2023");
}
