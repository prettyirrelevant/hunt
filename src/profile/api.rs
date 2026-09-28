use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::model::{Profile, Work};
use crate::ui::error::Error;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct You {
    pub profile: Option<Profile>,
    pub cv: Option<String>,
    pub work: Vec<WorkItem>,
    pub repo_roots: Vec<String>,
    pub about: String,
    /// One link per line.
    pub sites: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkItem {
    pub title: String,
    pub source: String,
    pub work: Option<Work>,
}

#[server]
pub async fn get_you() -> Result<You, Error> {
    use std::sync::Arc;

    use crate::app::App;

    let app = expect_context::<Arc<App>>();
    let settings = app.settings().await?;
    let notes = super::repo::notes(&app.db).await?;
    let cv = notes.iter().find(|n| n.source == "cv").map(|n| n.title.clone());
    let work = notes
        .into_iter()
        .filter_map(|n| {
            let title = match n.source.as_str() {
                "repo" => n.title,
                "site" => n.path.split(' ').next().unwrap_or_default().to_string(),
                _ => return None,
            };
            Some(WorkItem { title, source: n.source, work: n.facts.map(|f| f.0) })
        })
        .collect();
    Ok(You {
        profile: super::repo::profile(&app.db).await?,
        cv,
        work,
        repo_roots: settings.repo_roots.iter().map(|p| p.display().to_string()).collect(),
        about: super::repo::about(&app.db).await?.unwrap_or_default(),
        sites: settings.sites.join("\n"),
    })
}

#[server]
pub async fn save_profile(profile: Profile) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::{app::App, jobs};

    let app = expect_context::<Arc<App>>();
    super::repo::save_profile(&app.db, &profile).await?;
    jobs::record(&app.db, None, "profile", "You edited your profile", serde_json::json!({})).await?;
    Ok(())
}

#[server]
pub async fn save_context(about: String, sites: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::app::App;

    let mut links = vec![];
    for line in sites.lines().map(str::trim).filter(|l| !l.is_empty()) {
        match url::Url::parse(line) {
            Ok(link) if matches!(link.scheme(), "http" | "https") => links.push(link.to_string()),
            _ => return Err(Error(format!("{line} is not a web address. Start it with https://"))),
        }
    }
    let app = expect_context::<Arc<App>>();
    Ok(super::service::save_context(&app, about.trim().to_string(), links).await?)
}

#[server]
pub async fn read_repos_now() -> Result<(), Error> {
    use std::sync::Arc;

    use crate::app::App;

    Ok(expect_context::<Arc<App>>().queue(super::service::ReadRepos, "read_repos").await?)
}
