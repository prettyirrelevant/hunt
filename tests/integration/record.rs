use chrono::{Duration, Utc};
use hunt::jobs::{self, Posting, Stage};

fn posting(id: &str, company: &str, title: &str) -> Posting {
    Posting::new("test", id, company, title, "https://example.com")
}

#[tokio::test]
async fn the_same_role_from_two_boards_is_stored_once() {
    let db = crate::fresh_db().await;
    jobs::add(&db, &[posting("1", "Acme Inc.", "Senior Backend Engineer (Remote)")]).await.unwrap();
    let again = jobs::add(&db, &[posting("9", "ACME", "Backend Engineer, Senior")]).await.unwrap();
    assert!(again.is_empty());
}

#[tokio::test]
async fn the_same_role_twice_in_one_sweep_is_stored_once() {
    let db = crate::fresh_db().await;
    let ids =
        jobs::add(&db, &[posting("1", "Acme", "Backend Engineer"), posting("9", "Acme Ltd", "Engineer, Backend")])
            .await
            .unwrap();
    assert_eq!(ids.len(), 1);
}

#[tokio::test]
async fn a_different_city_is_a_different_role() {
    let db = crate::fresh_db().await;
    let ids = jobs::add(
        &db,
        &[posting("1", "Acme", "Solutions Engineer - Berlin"), posting("2", "Acme", "Solutions Engineer - Munich")],
    )
    .await
    .unwrap();
    assert_eq!(ids.len(), 2);
}

#[tokio::test]
async fn a_role_posted_again_weeks_later_is_flagged() {
    let db = crate::fresh_db().await;
    let mut first = posting("1", "Acme", "Platform Engineer");
    first.posted_at = Some(Utc::now() - Duration::days(40));
    let mut again = posting("2", "Acme", "Platform Engineer");
    again.posted_at = Some(Utc::now());

    let ids = jobs::add(&db, &[first]).await.unwrap();
    jobs::add(&db, &[again]).await.unwrap();
    assert_eq!(jobs::get(&db, ids[0]).await.unwrap().flags, vec!["reposted"]);
}

#[tokio::test]
async fn flags_are_added_once() {
    let db = crate::fresh_db().await;
    let ids = jobs::add(&db, &[posting("1", "Acme", "Engineer")]).await.unwrap();
    jobs::add_flags(&db, &[(ids[0], "stale"), (ids[0], "stale"), (ids[0], "video")]).await.unwrap();
    jobs::add_flags(&db, &[(ids[0], "video")]).await.unwrap();
    assert_eq!(jobs::get(&db, ids[0]).await.unwrap().flags, vec!["stale", "video"]);
}

#[tokio::test]
async fn moving_a_job_writes_the_record() {
    let db = crate::fresh_db().await;
    let ids = jobs::add(&db, &[posting("1", "Acme", "Engineer")]).await.unwrap();
    jobs::move_to(&db, &[(ids[0], "Not interested in fintech")], Stage::Skipped).await.unwrap();
    jobs::move_to(&db, &[(ids[0], "")], Stage::Skipped).await.unwrap();

    let bodies: Vec<String> =
        sqlx::query_scalar("select body from events where job_id = $1").bind(ids[0]).fetch_all(&db).await.unwrap();
    assert_eq!(bodies, vec!["Acme · Engineer: New → Skipped. Not interested in fintech"]);
}

#[tokio::test]
async fn moving_many_jobs_keeps_each_reason() {
    let db = crate::fresh_db().await;
    let ids = jobs::add(&db, &[posting("1", "Acme", "Engineer"), posting("2", "Globex", "Designer")]).await.unwrap();
    jobs::move_to(&db, &[(ids[0], "Office in Berlin"), (ids[1], "Office in Lagos")], Stage::Filtered).await.unwrap();

    let bodies: Vec<String> =
        sqlx::query_scalar("select body from events order by job_id").fetch_all(&db).await.unwrap();
    assert_eq!(
        bodies,
        vec![
            "Acme · Engineer: New → Not a fit. Office in Berlin",
            "Globex · Designer: New → Not a fit. Office in Lagos"
        ]
    );
}

#[tokio::test]
async fn a_move_from_a_stage_the_job_left_does_nothing() {
    let db = crate::fresh_db().await;
    let ids = jobs::add(&db, &[posting("1", "Acme", "Engineer")]).await.unwrap();
    jobs::move_to(&db, &[(ids[0], "You skipped it")], Stage::Skipped).await.unwrap();

    let moved = jobs::move_from(&db, Stage::New, &[(ids[0], "Scored 90")], Stage::Shortlist).await.unwrap();
    assert!(moved.is_empty());
    assert_eq!(jobs::get(&db, ids[0]).await.unwrap().stage, Stage::Skipped);
}
