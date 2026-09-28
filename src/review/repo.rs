use anyhow::Result;
use sqlx::{PgPool, types::Json};

use super::model::{Draft, Tailored};

pub async fn save(db: &PgPool, job_id: i64, tailored: &Tailored, email_to: Option<&str>, provider: &str) -> Result<()> {
    sqlx::query(
        "insert into drafts (job_id, cv, cv_changes, letter, email_to, email_subject, answers, provider)
         values ($1, $2, $3, $4, $5, $6, $7, $8)
         on conflict (job_id) do update set
             cv = $2, cv_changes = $3, letter = $4, email_to = $5, email_subject = $6, answers = $7,
             provider = $8, created_at = now()",
    )
    .bind(job_id)
    .bind(Json(&tailored.cv))
    .bind(Json(&tailored.changes))
    .bind(&tailored.letter)
    .bind(email_to)
    .bind(&tailored.email_subject)
    .bind(Json(&tailored.answers))
    .bind(provider)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get(db: &PgPool, job_id: i64) -> Result<Option<Draft>> {
    Ok(sqlx::query_as(
        "select job_id, cv, cv_changes, letter, email_to, email_subject, answers, provider, created_at
         from drafts where job_id = $1",
    )
    .bind(job_id)
    .fetch_optional(db)
    .await?)
}

pub async fn update_letter(db: &PgPool, job_id: i64, letter: &str) -> Result<()> {
    sqlx::query("update drafts set letter = $2 where job_id = $1").bind(job_id).bind(letter).execute(db).await?;
    Ok(())
}
