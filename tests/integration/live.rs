//! Hits every real source. Slow and network-bound, so it only runs on request:
//! `cargo test --features ssr --test integration live -- --ignored --nocapture`

use hunt::discovery::{
    model::{Reach, Want},
    sources::{Source, feeds::Feed, harvest},
};

#[tokio::test]
#[ignore = "calls every live job source"]
async fn every_source_answers() {
    let want = Want {
        terms: vec!["backend engineer".into(), "rust".into()],
        reach: Reach { country: "gb".into(), relocate: true },
        seniority: vec![],
        days: 14,
    };
    let http = reqwest::Client::builder().user_agent("hunt-live-test").build().unwrap();
    let mut failed = vec![];
    for source in std::iter::once(Source::Freehire).chain(Feed::ALL.into_iter().map(Source::Feed)) {
        let name = source.name();
        let result = harvest(&http, &want, std::slice::from_ref(&source)).await;
        println!(
            "{name:<18} {:>4} postings {}",
            result.postings.len(),
            result.failures.first().map_or("", |(_, e)| e.as_str())
        );
        if !result.failures.is_empty() {
            failed.push(name);
        }
    }
    assert!(failed.is_empty(), "sources failed: {failed:?}");
}
