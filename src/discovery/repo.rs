use std::collections::HashSet;

use anyhow::Result;
use pgvector::Vector;
use sqlx::PgPool;

pub async fn count(db: &PgPool) -> Result<i64> {
    Ok(sqlx::query_scalar("select count(*) from jobs").fetch_one(db).await?)
}

pub async fn save_embeddings(db: &PgPool, ids: &[i64], embeddings: &[Vector]) -> Result<()> {
    sqlx::query(
        "update jobs j set embedding = e.embedding
         from unnest($1::bigint[], $2::vector[]) as e (id, embedding)
         where j.id = e.id",
    )
    .bind(ids)
    .bind(embeddings)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn applied_recently(db: &PgPool, companies: &[&str]) -> Result<HashSet<String>> {
    Ok(sqlx::query_scalar(
        "select distinct lower(company) from jobs
         where lower(company) = any(select lower(c) from unnest($1::text[]) c)
           and stage in ('sending', 'applied', 'screen', 'interview', 'offer', 'rejected', 'ghosted')
           and stage_at > now() - interval '90 days'",
    )
    .bind(companies)
    .fetch_all(db)
    .await?
    .into_iter()
    .collect())
}
