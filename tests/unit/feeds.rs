use hunt::discovery::sources::{feeds::Feed, freehire};
use hunt::jobs::WorkMode;

use crate::fixture;

fn fixture_for(feed: Feed) -> &'static str {
    match feed {
        Feed::Himalayas => "himalayas.json",
        Feed::Jobicy => "jobicy.json",
        Feed::Remotive => "remotive.json",
        Feed::RemoteOk => "remoteok.json",
        Feed::WeWorkRemotely => "weworkremotely.rss",
        Feed::WorkingNomads => "workingnomads.json",
        Feed::Arbeitnow => "arbeitnow.json",
        Feed::LandingJobs => "landingjobs.json",
        Feed::FourDayWeek => "fourdayweek.json",
        Feed::HackerNews => "hackernews.json",
        Feed::LinkedIn => "linkedin.html",
    }
}

#[test]
fn every_feed_fixture_parses_into_postings() {
    for feed in Feed::ALL {
        let postings = feed.parse(&fixture(fixture_for(feed))).unwrap_or_else(|e| panic!("{}: {e:#}", feed.name()));
        assert!(!postings.is_empty(), "{} gave no postings", feed.name());
        for p in &postings {
            assert!(!p.title.is_empty(), "{}: empty title in {p:?}", feed.name());
            assert!(!p.company.is_empty(), "{}: empty company in {p:?}", feed.name());
            assert!(p.url.starts_with("https://"), "{}: bad url {}", feed.name(), p.url);
        }
    }
}

#[test]
fn we_work_remotely_splits_company_from_role() {
    let p = &Feed::WeWorkRemotely.parse(&fixture("weworkremotely.rss")).unwrap()[0];
    assert_eq!((p.company.as_str(), p.title.as_str()), ("Fin", "Solutions Architect -Spanish Speaking"));
    assert_eq!(p.regions, vec!["global"]);
}

#[test]
fn hacker_news_comments_become_roles() {
    let p = &Feed::HackerNews.parse(&fixture("hackernews.json")).unwrap()[0];
    assert_eq!((p.company.as_str(), p.title.as_str()), ("Modash.io", "Senior Product Engineer"));
    assert_eq!((p.work_mode, p.regions.clone()), (WorkMode::Remote, vec!["eu".to_string()]));
}

#[test]
fn linkedin_cards_become_roles() {
    let p = &Feed::LinkedIn.parse(&fixture("linkedin.html")).unwrap()[0];
    assert_eq!((p.company.as_str(), p.title.as_str()), ("Kraken", "Senior Software Engineer - Rust - Core Services"));
    assert!(!p.url.contains('?'));
}

#[test]
fn freehire_jobs_keep_their_tags() {
    let p = &freehire::parse(&fixture("freehire.json")).unwrap()[0];
    assert!(p.key.starts_with("freehire:"));
    assert_eq!(p.work_mode, WorkMode::Remote);
    assert!(p.regions.contains(&"global".to_string()));
    assert!(!p.requirements.is_empty());
    assert!(!p.url.contains("utm_source"));
}
