use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ui::error::Error;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct View {
    pub country: String,
    pub relocate: bool,
    pub contact: Contact,
    pub providers: Vec<Provider>,
    pub email: String,
    pub has_password: bool,
    pub backup_dir: String,
    pub repo_roots: String,
    pub ignore: String,
    pub watched: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub links: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub on: bool,
    pub status: String,
    /// The model it reads your sites with. Empty means its default.
    pub web_model: String,
    pub models: Vec<String>,
}

#[server]
pub async fn get_settings() -> Result<View, Error> {
    use std::sync::Arc;

    use crate::{
        app::App,
        common::{
            ai::{self, Status},
            mail::Mailbox,
        },
    };

    let app = expect_context::<Arc<App>>();
    let s = app.settings().await?;
    let status = |p: ai::Provider| match app.ai.status(p) {
        Status::Ready => "ready".to_string(),
        Status::Resting { minutes_left } => format!("resting for {minutes_left} min after a usage limit"),
        Status::Missing => "not installed".to_string(),
    };
    let listed = futures::future::join_all(ai::Provider::ALL.map(|p| app.ai.models(p))).await;
    let mut providers: Vec<Provider> = ai::Provider::ALL
        .into_iter()
        .zip(listed)
        .map(|(p, models)| Provider {
            name: p.name().into(),
            on: s.providers.contains(&p),
            status: status(p),
            web_model: s.web_models.get(&p).cloned().unwrap_or_default(),
            models,
        })
        .collect();
    // Turned-on providers first, in your order.
    providers.sort_by_key(|p| s.providers.iter().position(|q| q.name() == p.name).unwrap_or(usize::MAX));
    let email = s.email.clone().unwrap_or_default();
    Ok(View {
        country: s.reach.as_ref().map(|r| r.country.clone()).unwrap_or_default(),
        relocate: s.reach.as_ref().is_none_or(|r| r.relocate),
        contact: Contact {
            name: s.contact.name,
            email: s.contact.email,
            phone: s.contact.phone,
            location: s.contact.location,
            links: s.contact.links.join("\n"),
        },
        providers,
        has_password: !email.is_empty() && Mailbox::open(&app.config, &email).is_ok(),
        email,
        backup_dir: app.config.backups(s.backup_dir.as_deref()).display().to_string(),
        repo_roots: s.repo_roots.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n"),
        ignore: s.ignore.join("\n"),
        watched: s.watched.iter().map(ToString::to_string).collect(),
    })
}

#[server]
pub async fn save_reach(country: String, relocate: bool) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::config::Settings;
    use crate::{
        app::App,
        discovery::{model::Reach, service::Sweep},
    };

    let app = expect_context::<Arc<App>>();
    let first =
        Settings::edit(&app.db, |s| s.reach.replace(Reach { country: country.to_lowercase(), relocate }).is_none())
            .await?;
    if first {
        app.queue(Sweep, "sweep").await?;
    }
    Ok(())
}

#[server]
pub async fn save_contact(contact: Contact) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::app::App;
    use crate::config::Settings;

    let app = expect_context::<Arc<App>>();
    let contact = crate::config::Contact {
        name: contact.name,
        email: contact.email,
        phone: contact.phone,
        location: contact.location,
        links: contact.links.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect(),
    };
    Ok(Settings::edit(&app.db, |s| s.contact = contact).await?)
}

/// `order` lists the providers you turned on, first choice first.
#[server]
pub async fn save_providers(order: Vec<String>) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::config::Settings;
    use crate::{app::App, common::ai::Provider};

    let app = expect_context::<Arc<App>>();
    let providers = order.iter().filter_map(|name| Provider::ALL.into_iter().find(|p| p.name() == name)).collect();
    Ok(Settings::edit(&app.db, |s| s.providers = providers).await?)
}

/// Saves `model` after one test call. Empty means the provider's default.
#[server]
pub async fn save_web_model(provider: String, model: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::{app::App, common::ai, config::Settings};

    let app = expect_context::<Arc<App>>();
    let Some(provider) = ai::Provider::ALL.into_iter().find(|p| p.name() == provider) else {
        return Err(Error(format!("hunt does not know the provider {provider}")));
    };
    if !model.is_empty() {
        app.ai
            .check(provider, &model)
            .await
            .map_err(|err| Error(format!("{} could not use {model}: {err:#}", provider.name())))?;
    }
    Ok(Settings::edit(&app.db, |s| {
        if model.is_empty() {
            s.web_models.remove(&provider);
        } else {
            s.web_models.insert(provider, model);
        }
    })
    .await?)
}

/// The password goes to the Keychain, the address to settings.
#[server]
pub async fn save_email(address: String, password: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::config::Settings;
    use crate::{app::App, common::mail::Mailbox};

    let app = expect_context::<Arc<App>>();
    if !password.is_empty() {
        Mailbox::remember(&app.config, &address, &password.replace(' ', ""))?;
    }
    Ok(Settings::edit(&app.db, |s| s.email = Some(address).filter(|a| !a.is_empty())).await?)
}

#[server]
pub async fn save_folders(backup_dir: String, repo_roots: String, ignore: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::app::App;
    use crate::config::Settings;

    let app = expect_context::<Arc<App>>();
    Ok(Settings::edit(&app.db, |s| {
        s.backup_dir = Some(backup_dir.trim()).filter(|dir| !dir.is_empty()).map(Into::into);
        s.repo_roots = repo_roots.lines().map(str::trim).filter(|l| !l.is_empty()).map(Into::into).collect();
        s.ignore = ignore.lines().map(str::trim).filter(|l| !l.is_empty()).map(Into::into).collect();
    })
    .await?)
}

#[server]
pub async fn watch(link: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::config::Settings;
    use crate::{app::App, discovery::sources::ats::Board};

    let app = expect_context::<Arc<App>>();
    let board = Board::from_url(&link)
        .ok_or_else(|| Error("hunt does not recognise that careers link. It reads Greenhouse, Lever, Ashby, Workable, SmartRecruiters, Recruitee, Rippling, Breezy and Workday boards.".into()))?;
    Ok(Settings::edit(&app.db, |s| {
        if !s.watched.contains(&board) {
            s.watched.push(board);
        }
    })
    .await?)
}

#[server]
pub async fn unwatch(name: String) -> Result<(), Error> {
    use std::sync::Arc;

    use crate::app::App;
    use crate::config::Settings;

    let app = expect_context::<Arc<App>>();
    Ok(Settings::edit(&app.db, |s| s.watched.retain(|board| board.to_string() != name)).await?)
}
