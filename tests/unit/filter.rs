use chrono::Utc;
use hunt::discovery::filter::{check, places};
use hunt::discovery::model::{Reach, Verdict};
use hunt::jobs::{Job, Stage, WorkMode};

fn job(mode: WorkMode, regions: &[&str], countries: &[&str], description: &str) -> Job {
    Job {
        id: 1,
        source: "test".into(),
        url: String::new(),
        apply_url: String::new(),
        company: "Acme".into(),
        title: "Backend Engineer".into(),
        location: String::new(),
        work_mode: mode,
        regions: regions.iter().map(std::string::ToString::to_string).collect(),
        countries: countries.iter().map(std::string::ToString::to_string).collect(),
        visa: None,
        relocation: None,
        salary_min: Some(100_000),
        salary_max: None,
        currency: None,
        seniority: None,
        skills: vec![],
        requirements: vec![],
        description: description.into(),
        posted_at: None,
        first_seen: Utc::now(),
        stage: Stage::New,
        stage_at: Utc::now(),
        fit: None,
        like_score: None,
        odds: None,
        score: None,
        assessment: None,
        flags: vec![],
        apply_via: None,
    }
}

fn lagos() -> Reach {
    Reach { country: "ng".into(), relocate: true }
}

#[test]
fn worldwide_remote_is_kept() {
    assert_eq!(check(&job(WorkMode::Remote, &["global"], &[], ""), &lagos()), Verdict::Keep { flags: vec![] });
}

#[test]
fn remote_in_another_region_is_dropped() {
    let verdict = check(&job(WorkMode::Remote, &["north_america"], &["us"], ""), &lagos());
    assert_eq!(verdict, Verdict::Drop("Remote only for people in US, North America".into()));
}

#[test]
fn remote_in_your_region_is_kept() {
    assert!(matches!(check(&job(WorkMode::Remote, &["africa"], &[], ""), &lagos()), Verdict::Keep { .. }));
}

#[test]
fn remote_text_that_locks_the_country_is_dropped() {
    let verdict = check(&job(WorkMode::Remote, &[], &[], "You must be based in the United States."), &lagos());
    assert!(matches!(verdict, Verdict::Drop(_)));
}

#[test]
fn office_job_abroad_with_a_visa_is_kept() {
    let mut j = job(WorkMode::Onsite, &["eu"], &["nl"], "");
    j.visa = Some(true);
    assert!(matches!(check(&j, &lagos()), Verdict::Keep { .. }));
}

#[test]
fn office_job_abroad_without_a_visa_is_dropped() {
    assert!(matches!(check(&job(WorkMode::Onsite, &["eu"], &["nl"], ""), &lagos()), Verdict::Drop(_)));
}

#[test]
fn a_video_step_goes_to_you() {
    let verdict =
        check(&job(WorkMode::Remote, &["global"], &[], "Please record a short video introduction."), &lagos());
    assert_eq!(verdict, Verdict::Manual("Asks for a recorded video".into()));
}

#[test]
fn clearance_roles_are_dropped_outside_the_us() {
    let verdict = check(&job(WorkMode::Remote, &["global"], &[], "Active security clearance required."), &lagos());
    assert!(matches!(verdict, Verdict::Drop(_)));
}

#[test]
fn missing_salary_is_flagged() {
    let mut j = job(WorkMode::Remote, &["global"], &[], "");
    j.salary_min = None;
    assert_eq!(check(&j, &lagos()), Verdict::Keep { flags: vec!["no_salary"] });
}

#[test]
fn free_text_locations_become_regions() {
    assert_eq!(places("Remote - Worldwide"), (vec!["global".into()], vec![]));
    assert_eq!(places("Remote (EMEA)").0, vec!["eu", "uk", "africa", "mena"]);
    assert_eq!(places("New York, United States"), (vec!["north_america".into()], vec!["us".into()]));
    assert_eq!(places("Business Development"), (vec![], vec![]));
}
