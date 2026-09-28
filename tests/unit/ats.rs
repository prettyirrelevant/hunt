use hunt::discovery::sources::ats::{Ats, Board};
use hunt::jobs::{Posting, WorkMode};

use crate::fixture;

fn parse(ats: Ats, slug: &str, name: &str) -> Vec<Posting> {
    let board = Board { ats, slug: slug.into(), company: slug.into() };
    board.parse(&fixture(name)).unwrap()
}

#[test]
fn careers_links_resolve_to_boards() {
    let board = Board::from_url("https://jobs.ashbyhq.com/linear/abc-123").unwrap();
    assert_eq!((board.ats, board.slug.as_str(), board.company.as_str()), (Ats::Ashby, "linear", "Linear"));

    let board = Board::from_url("https://boards-api.greenhouse.io/v1/boards/scale-ai/jobs").unwrap();
    assert_eq!((board.ats, board.company.as_str()), (Ats::Greenhouse, "Scale Ai"));

    let board = Board::from_url("https://channable.recruitee.com/o/some-role").unwrap();
    assert_eq!((board.ats, board.slug.as_str()), (Ats::Recruitee, "channable"));

    assert!(Board::from_url("https://example.com/careers").is_none());
}

#[test]
fn every_ats_fixture_parses_into_postings() {
    for (ats, slug, name) in [
        (Ats::Greenhouse, "vercel", "greenhouse.json"),
        (Ats::Lever, "spotify", "lever.json"),
        (Ats::Ashby, "linear", "ashby.json"),
        (Ats::SmartRecruiters, "Ubisoft2", "smartrecruiters.json"),
        (Ats::Recruitee, "channable", "recruitee.json"),
        (Ats::Rippling, "rippling", "rippling.json"),
        (Ats::Breezy, "breezy", "breezy.json"),
        (Ats::Workday, "nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite", "workday.json"),
    ] {
        let postings = parse(ats, slug, name);
        assert!(!postings.is_empty(), "{name} gave no postings");
        for p in &postings {
            assert!(!p.title.is_empty() && p.url.starts_with("https://"), "{name}: {p:?}");
        }
    }
}

#[test]
fn ashby_remote_roles_are_remote() {
    let p = &parse(Ats::Ashby, "linear", "ashby.json")[0];
    assert_eq!((p.work_mode, p.regions.clone()), (WorkMode::Remote, vec!["eu".to_string()]));
}

#[test]
fn lever_office_roles_keep_their_city() {
    let p = &parse(Ats::Lever, "spotify", "lever.json")[0];
    assert_eq!((p.work_mode, p.location.as_str()), (WorkMode::Onsite, "London"));
}

#[test]
fn recruitee_monthly_pay_is_not_read_as_yearly() {
    let p = &parse(Ats::Recruitee, "channable", "recruitee.json")[0];
    assert_eq!((p.work_mode, p.salary_min), (WorkMode::Hybrid, None));
}
