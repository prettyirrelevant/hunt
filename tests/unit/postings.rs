use hunt::jobs::Posting;

fn posting(url: &str, apply_url: &str) -> Posting {
    Posting { apply_url: apply_url.into(), ..Posting::new("feed", 1, "Acme", "Engineer", url) }
}

#[test]
fn a_posting_with_web_links_is_kept_as_it_is() {
    let kept = posting("https://acme.dev/jobs/1", "http://apply.acme.dev/1").with_web_links().expect("kept");
    assert_eq!(kept.url, "https://acme.dev/jobs/1");
    assert_eq!(kept.apply_url, "http://apply.acme.dev/1");
}

#[test]
fn a_posting_whose_link_is_a_script_is_dropped() {
    assert!(posting("javascript:alert(1)", "https://acme.dev/apply").with_web_links().is_none());
}

#[test]
fn an_apply_link_that_is_not_on_the_web_falls_back_to_the_posting() {
    for apply_url in ["javascript:alert(1)", "mailto:jobs@acme.dev", ""] {
        let kept = posting("https://acme.dev/jobs/1", apply_url).with_web_links().expect("kept");
        assert_eq!(kept.apply_url, "https://acme.dev/jobs/1");
    }
}
