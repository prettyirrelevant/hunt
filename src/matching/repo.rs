use std::collections::HashMap;

use anyhow::Result;
use pgvector::Vector;
use sqlx::{PgPool, types::Json};

use super::{learn::Model, model::Assessment};
use crate::jobs::Job;

pub enum Label {
    /// You approved or applied (true) or skipped (false).
    Like,
    /// The company moved you forward (true) or said no or went quiet (false).
    Odds,
}

pub async fn labelled(db: &PgPool, label: Label) -> Result<Vec<(Job, Vec<f32>, bool)>> {
    let sql = match label {
        Label::Like => {
            "select id, embedding, stage <> 'skipped' from jobs
             where embedding is not null
               and stage in ('approved', 'skipped', 'sending', 'applied', 'screen', 'interview', 'offer', 'rejected', 'ghosted', 'withdrawn')"
        }
        Label::Odds => {
            "select id, embedding, stage in ('screen', 'interview', 'offer') from jobs
             where embedding is not null
               and (stage in ('screen', 'interview', 'offer', 'rejected', 'ghosted')
                    or (stage = 'applied' and stage_at < now() - interval '21 days'))"
        }
    };
    let rows: Vec<(i64, Vector, bool)> = sqlx::query_as(sql).fetch_all(db).await?;
    let ids: Vec<i64> = rows.iter().map(|(id, _, _)| *id).collect();
    let mut jobs: HashMap<i64, Job> = sqlx::query_as::<_, Job>(crate::select_jobs!("where id = any($1)"))
        .bind(&ids)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|job| (job.id, job))
        .collect();
    Ok(rows
        .into_iter()
        .filter_map(|(id, embedding, label)| Some((jobs.remove(&id)?, embedding.to_vec(), label)))
        .collect())
}

pub async fn embedding(db: &PgPool, id: i64) -> Result<Option<Vec<f32>>> {
    let vector: Option<Vector> =
        sqlx::query_scalar("select embedding from jobs where id = $1").bind(id).fetch_one(db).await?;
    Ok(vector.map(|v| v.to_vec()))
}

pub async fn centroids(db: &PgPool) -> Result<(Option<Vec<f32>>, Option<Vec<f32>>, i64)> {
    let (liked, skipped, skips): (Option<Vector>, Option<Vector>, i64) = sqlx::query_as(
        "select avg(embedding) filter (where stage <> 'skipped'),
                avg(embedding) filter (where stage = 'skipped'),
                count(*) filter (where stage = 'skipped')
         from jobs
         where embedding is not null
           and stage in ('approved', 'skipped', 'sending', 'applied', 'screen', 'interview', 'offer', 'rejected', 'ghosted', 'withdrawn')",
    )
    .fetch_one(db)
    .await?;
    Ok((liked.map(|v| v.to_vec()), skipped.map(|v| v.to_vec()), skips))
}

pub async fn model(db: &PgPool, name: &str) -> Result<Option<Model>> {
    let data: Option<Json<Model>> =
        sqlx::query_scalar("select data from models where name = $1").bind(name).fetch_optional(db).await?;
    Ok(data.map(|Json(m)| m))
}

pub async fn save_model(db: &PgPool, name: &str, model: &Model) -> Result<()> {
    sqlx::query(
        "insert into models (name, data) values ($1, $2)
         on conflict (name) do update set data = $2, trained_at = now()",
    )
    .bind(name)
    .bind(Json(model))
    .execute(db)
    .await?;
    Ok(())
}

pub async fn save(
    db: &PgPool,
    id: i64,
    assessment: &Assessment,
    like: Option<f64>,
    odds: Option<f64>,
    score: i32,
) -> Result<()> {
    let apply_via = if assessment.apply_email.is_some() { "email" } else { "form" };
    sqlx::query(
        "update jobs set fit = $2, like_score = $3, odds = $4, score = $5, assessment = $6, apply_via = $7
         where id = $1",
    )
    .bind(id)
    .bind(i32::from(assessment.fit))
    .bind(like.map(|v| v as f32))
    .bind(odds.map(|v| v as f32))
    .bind(score)
    .bind(Json(assessment))
    .bind(apply_via)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn assessed_today(db: &PgPool) -> Result<i64> {
    Ok(sqlx::query_scalar("select count(*) from events where kind = 'score' and at > date_trunc('day', now())")
        .fetch_one(db)
        .await?)
}
