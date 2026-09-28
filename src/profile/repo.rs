use anyhow::Result;
use pgvector::Vector;
use sqlx::{PgPool, types::Json};

use super::model::{Note, Profile, Work};

pub async fn upsert_notes(db: &PgPool, source: &str, notes: &[(String, String, String)]) -> Result<Vec<i64>> {
    let (mut paths, mut titles, mut bodies) = (vec![], vec![], vec![]);
    for (path, title, body) in notes {
        paths.push(path.as_str());
        titles.push(title.as_str());
        bodies.push(body.as_str());
    }
    Ok(sqlx::query_scalar(
        "with saved as (
             insert into notes (source, path, title, body)
             select $1, * from unnest($2::text[], $3::text[], $4::text[])
             on conflict (source, path) do update
                 set title = excluded.title, body = excluded.body, updated_at = now(),
                     summary = case when notes.body = excluded.body then notes.summary end
             returning id, summary is null as stale
         )
         select id from saved where stale",
    )
    .bind(source)
    .bind(&paths)
    .bind(&titles)
    .bind(&bodies)
    .fetch_all(db)
    .await?)
}

pub async fn remove_notes(db: &PgPool, ids: &[i64]) -> Result<()> {
    sqlx::query("delete from notes where id = any($1)").bind(ids).execute(db).await?;
    Ok(())
}

pub async fn replace_site_notes(
    db: &PgPool,
    listed: &[String],
    read: &[&str],
    notes: &[(String, Work, Vector)],
) -> Result<()> {
    let mut tx = db.begin().await?;
    sqlx::query(
        "delete from notes
         where source = 'site' and (split_part(path, ' ', 1) <> all($1) or split_part(path, ' ', 1) = any($2))",
    )
    .bind(listed)
    .bind(read)
    .execute(&mut *tx)
    .await?;

    let (mut paths, mut titles, mut summaries, mut facts, mut embeddings) = (vec![], vec![], vec![], vec![], vec![]);
    for (path, work, embedding) in notes {
        paths.push(path.as_str());
        titles.push(work.title.as_str());
        summaries.push(work.summary.as_str());
        facts.push(serde_json::to_value(work)?);
        embeddings.push(embedding.clone());
    }
    sqlx::query(
        "insert into notes (source, path, title, body, summary, facts, embedding)
         select 'site', path, title, summary, summary, facts, embedding
         from unnest($1::text[], $2::text[], $3::text[], $4::jsonb[], $5::vector[])
             as n (path, title, summary, facts, embedding)",
    )
    .bind(&paths)
    .bind(&titles)
    .bind(&summaries)
    .bind(&facts)
    .bind(&embeddings)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn about(db: &PgPool) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("select body from notes where source = 'about'").fetch_optional(db).await?)
}

pub async fn note_body(db: &PgPool, id: i64) -> Result<(String, String)> {
    Ok(sqlx::query_as("select title, body from notes where id = $1").bind(id).fetch_one(db).await?)
}

pub async fn save_work(db: &PgPool, id: i64, work: &Work, embedding: Vector) -> Result<()> {
    sqlx::query("update notes set summary = $2, facts = $3, embedding = $4 where id = $1")
        .bind(id)
        .bind(&work.summary)
        .bind(Json(work))
        .bind(embedding)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn notes(db: &PgPool) -> Result<Vec<Note>> {
    Ok(sqlx::query_as(
        "select id, source, path, title, summary, facts, updated_at from notes order by source, updated_at desc",
    )
    .fetch_all(db)
    .await?)
}

pub async fn cv_text(db: &PgPool) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("select body from notes where source = 'cv' order by updated_at desc limit 1")
        .fetch_optional(db)
        .await?)
}

pub async fn profile(db: &PgPool) -> Result<Option<Profile>> {
    let data: Option<Json<Profile>> =
        sqlx::query_scalar("select data from profile where id = 1").fetch_optional(db).await?;
    Ok(data.map(|Json(p)| p))
}

pub async fn save_profile(db: &PgPool, profile: &Profile) -> Result<()> {
    sqlx::query(
        "insert into profile (id, data) values (1, $1)
         on conflict (id) do update set data = $1, updated_at = now()",
    )
    .bind(Json(profile))
    .execute(db)
    .await?;
    Ok(())
}

pub async fn evidence(db: &PgPool, needs: &[String], meanings: &[Vector], limit: i64) -> Result<Vec<Work>> {
    let any_word: Vec<String> =
        needs.iter().map(|need| need.split_whitespace().collect::<Vec<_>>().join(" or ")).collect();
    let rows: Vec<Json<Work>> = sqlx::query_scalar(
        "select found.facts
         from unnest($1::text[], $2::vector[]) with ordinality as need (words, meaning, n)
         cross join lateral (
             select n.facts, coalesce(1.0 / (60 + w.rank), 0) + coalesce(1.0 / (60 + m.rank), 0) as fused
             from notes n
             left join (
                 select id, row_number() over (order by ts_rank_cd(search, websearch_to_tsquery('english', need.words)) desc) as rank
                 from notes where facts is not null and search @@ websearch_to_tsquery('english', need.words)
                 limit 20
             ) w on w.id = n.id
             left join (
                 select id, row_number() over (order by embedding <=> need.meaning) as rank
                 from notes where facts is not null and embedding is not null
                 order by embedding <=> need.meaning
                 limit 20
             ) m on m.id = n.id
             where w.id is not null or m.id is not null
             order by fused desc
             limit $3
         ) found
         order by need.n, found.fused desc",
    )
    .bind(&any_word)
    .bind(meanings)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|Json(w)| w).collect())
}
