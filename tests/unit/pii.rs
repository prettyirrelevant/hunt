use std::sync::LazyLock;

use hunt::common::pii::Redactor;

static REDACTOR: LazyLock<Redactor> = LazyLock::new(|| {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pii");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(Redactor::fetch(&reqwest::Client::new(), &dir)).expect("model loads")
});

#[test]
fn names_and_addresses_are_removed() {
    let text = "Worked with Chidi Okafor on the payments ledger. Office at 12 Marina Road, Lagos.";
    let redacted = REDACTOR.redact(text).unwrap();
    assert!(!redacted.contains("Chidi Okafor"), "{redacted}");
    assert!(!redacted.contains("12 Marina Road"), "{redacted}");
    assert!(redacted.contains("payments ledger"), "{redacted}");
}

#[test]
fn companies_and_technology_stay() {
    let text = "Built the Stripe integration in Rust and cut checkout latency by 40%.";
    assert_eq!(REDACTOR.redact(text).unwrap(), text);
}
