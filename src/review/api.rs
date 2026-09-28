use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::model::{Answer, Change, Cv};
use crate::{
    jobs::{Job, Stage},
    matching::model::Assessment,
    ui::error::Error,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub title: String,
    pub company: String,
    pub place: String,
    pub score: Option<i32>,
    pub stage: Stage,
    pub by_email: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftView {
    pub job: Job,
    pub assessment: Option<Assessment>,
    pub cv: Cv,
    pub changes: Vec<Change>,
    pub letter: String,
    pub email_to: Option<String>,
    pub email_subject: Option<String>,
    pub answers: Vec<Answer>,
    pub provider: String,
}

#[server]
pub async fn get_batch() -> Result<Vec<Item>, Error> {
    use std::sync::Arc;

    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    let jobs: Vec<Job> = sqlx::query_as(crate::select_jobs!(
        "where stage in ('ready', 'approved', 'sending') order by score desc nulls last, id limit 100"
    ))
    .fetch_all(&app.db)
    .await?;
    Ok(jobs
        .into_iter()
        .map(|j| Item {
            id: j.id,
            place: j.where_label(),
            by_email: j.apply_via.as_deref() == Some("email"),
            title: j.title,
            company: j.company,
            score: j.score,
            stage: j.stage,
        })
        .collect())
}

#[server]
pub async fn get_draft(id: i64) -> Result<DraftView, Error> {
    use std::sync::Arc;

    use anyhow::Context;

    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    let job = crate::jobs::get(&app.db, id).await?;
    let draft = super::repo::get(&app.db, id).await?.context("no draft for this job yet")?;
    Ok(DraftView {
        assessment: job.assessment.clone().and_then(|a| serde_json::from_value(a).ok()),
        job,
        cv: draft.cv.0,
        changes: draft.cv_changes.0,
        letter: draft.letter,
        email_to: draft.email_to,
        email_subject: draft.email_subject,
        answers: draft.answers.0,
        provider: draft.provider,
    })
}

/// Keeps your edit and rebuilds the cover letter PDF from it.
#[server]
pub async fn save_letter(id: i64, letter: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    super::repo::update_letter(&app.db, id, &letter).await?;
    let job = crate::jobs::get(&app.db, id).await?;
    let contact = app.settings().await?.contact;
    let pdf = tokio::task::spawn_blocking(move || super::documents::letter_pdf(&contact, &job.company, &letter))
        .await
        .map_err(anyhow::Error::from)??;
    tokio::fs::write(app.config.documents().join(id.to_string()).join("cover-letter.pdf"), pdf)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(())
}

#[server]
pub async fn send_batch() -> Result<usize, Error> {
    use std::sync::Arc;

    use crate::app::App;

    Ok(crate::applying::service::send_batch(&expect_context::<Arc<App>>()).await?)
}

#[server]
pub async fn undo_send() -> Result<usize, Error> {
    use std::sync::Arc;

    use crate::app::App;

    Ok(crate::applying::service::undo(&expect_context::<Arc<App>>()).await?)
}
