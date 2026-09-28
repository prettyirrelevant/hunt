use hunt::{
    config::Settings,
    profile::{model::Work, repo},
};
use pgvector::Vector;
use sqlx::PgPool;

fn work(title: &str) -> Work {
    Work { title: title.into(), summary: format!("{title} summary"), highlights: vec![], stack: vec![] }
}

async fn site_notes(db: &PgPool) -> Vec<String> {
    sqlx::query_scalar("select path from notes where source = 'site' order by path").fetch_all(db).await.unwrap()
}

#[tokio::test]
async fn reading_a_site_replaces_only_its_own_notes() {
    let db = crate::fresh_db().await;
    let (a, b) = ("https://a.dev/".to_string(), "https://b.dev/".to_string());
    let v = || Vector::from(vec![0.1; 256]);
    let first = [(format!("{a} 0"), work("Old A"), v()), (format!("{b} 0"), work("B"), v())];
    repo::replace_site_notes(&db, &[a.clone(), b.clone()], &[&a, &b], &first).await.unwrap();

    let second = [(format!("{a} 0"), work("New A"), v()), (format!("{a} 1"), work("Another A"), v())];
    repo::replace_site_notes(&db, &[a.clone(), b.clone()], &[&a], &second).await.unwrap();
    assert_eq!(site_notes(&db).await, vec![format!("{a} 0"), format!("{a} 1"), format!("{b} 0")]);
}

#[tokio::test]
async fn a_removed_site_is_forgotten() {
    let db = crate::fresh_db().await;
    let (a, b) = ("https://a.dev/".to_string(), "https://b.dev/".to_string());
    let notes = [(format!("{b} 0"), work("B"), Vector::from(vec![0.1; 256]))];
    repo::replace_site_notes(&db, &[a.clone(), b.clone()], &[&b], &notes).await.unwrap();

    repo::replace_site_notes(&db, std::slice::from_ref(&a), &[], &[]).await.unwrap();
    assert!(site_notes(&db).await.is_empty());
}

#[tokio::test]
async fn edits_at_the_same_time_both_land() {
    let db = crate::fresh_db().await;
    let sites = Settings::edit(&db, |s| s.sites = vec!["https://a.dev/".into()]);
    let ignore = Settings::edit(&db, |s| s.ignore = vec!["client-work/".into()]);
    let (sites, ignore) = tokio::join!(sites, ignore);
    sites.unwrap();
    ignore.unwrap();

    let saved = Settings::load(&db).await.unwrap();
    assert_eq!(saved.sites, vec!["https://a.dev/"]);
    assert_eq!(saved.ignore, vec!["client-work/"]);
}
