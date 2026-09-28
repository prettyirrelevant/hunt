use leptos::prelude::*;

use super::model::{Insights, Log, Pipeline, Today};
use crate::ui::error::Error;

pub const LOG_PAGE: i64 = 50;

#[server]
pub async fn get_today() -> Result<Today, Error> {
    use std::sync::Arc;

    use super::{model::Setup, repo};
    use crate::{app::App, common::ai::Status, profile};

    let app = expect_context::<Arc<App>>();
    let settings = app.settings().await?;
    let mut today = repo::today(&app.db).await?;
    today.setup = Setup {
        reach: settings.reach.is_some(),
        cv: profile::repo::cv_text(&app.db).await?.is_some(),
        profile: profile::repo::profile(&app.db).await?.is_some_and(|p| !p.roles.is_empty()),
        ai: settings.providers.iter().any(|p| !matches!(app.ai.status(*p), Status::Missing)),
        email: settings.email.is_some(),
    };
    Ok(today)
}

#[server]
pub async fn get_pipeline() -> Result<Pipeline, Error> {
    use std::sync::Arc;

    use super::{model::flows, repo};
    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    Ok(Pipeline { flows: flows(&repo::reached(&app.db).await?), active: repo::active(&app.db).await? })
}

#[server]
pub async fn get_insights() -> Result<Insights, Error> {
    use std::sync::Arc;

    use super::repo;
    use crate::{app::App, profile};

    let app = expect_context::<Arc<App>>();
    let skills = profile::repo::profile(&app.db).await?.unwrap_or_default().skills;
    Ok(Insights {
        weeks: repo::weeks(&app.db).await?,
        bands: repo::bands(&app.db).await?,
        sources: repo::sources(&app.db).await?,
        drops: repo::drops(&app.db).await?,
        gaps: repo::gaps(&app.db, &skills).await?,
        median_reply_days: repo::median_reply_days(&app.db).await?,
        like: repo::learned(&app.db, "like").await?,
        odds: repo::learned(&app.db, "odds").await?,
        decisions: repo::decisions(&app.db).await?,
    })
}

#[server]
pub async fn get_log(kind: String, page: i64) -> Result<Log, Error> {
    use std::sync::Arc;

    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    Ok(super::repo::log(&app.db, &kind, page.max(1), LOG_PAGE).await?)
}
