use std::sync::Arc;

use anyhow::Result;
use graphile_worker::{IntoTaskHandlerResult, TaskHandler, WorkerContext};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{app::App, config::Config, insights, jobs};

/// Nightly dumps kept in the backup folder.
const KEEP: usize = 14;

pub async fn backup(app: &Arc<App>) -> Result<()> {
    let dir = app.config.backups(app.settings().await?.backup_dir.as_deref());
    let path = app.server.backup(&dir).await?;
    let size = tokio::fs::metadata(&path).await?.len();

    let mut dumps = vec![];
    let mut entries = tokio::fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.path().extension().is_some_and(|x| x == "dump") {
            dumps.push(entry.path());
        }
    }
    dumps.sort();
    for old in dumps.iter().rev().skip(KEEP) {
        tokio::fs::remove_file(old).await?;
    }
    let body = format!("Backup saved to {} ({:.1} MB)", path.display(), size as f64 / 1e6);
    jobs::record(&app.db, None, "system", &body, json!({ "path": path })).await
}

/// launchd on macOS, systemd on Linux, the Run key on Windows.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn install(config: &Config) -> Result<()> {
    use service_manager::{RestartPolicy, ServiceInstallCtx, ServiceStartCtx};

    let manager = user_services()?;
    manager.install(ServiceInstallCtx {
        label: label(config).parse()?,
        program: std::env::current_exe()?,
        args: start_args(config),
        contents: None,
        username: None,
        working_directory: None,
        environment: None,
        autostart: true,
        restart_policy: RestartPolicy::Always { delay_secs: Some(5) },
    })?;
    manager.start(ServiceStartCtx { label: label(config).parse()? })?;
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn uninstall(config: &Config) -> Result<()> {
    use service_manager::{ServiceStopCtx, ServiceUninstallCtx};

    let manager = user_services()?;
    manager.stop(ServiceStopCtx { label: label(config).parse()? }).ok();
    manager.uninstall(ServiceUninstallCtx { label: label(config).parse()? })?;
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn user_services() -> Result<Box<dyn service_manager::ServiceManager>> {
    let mut manager = <dyn service_manager::ServiceManager>::native()?;
    manager.set_level(service_manager::ServiceLevel::User)?;
    Ok(manager)
}

#[cfg(windows)]
pub fn install(config: &Config) -> Result<()> {
    windows_launch(config)?.enable()?;
    Ok(())
}

#[cfg(windows)]
pub fn uninstall(config: &Config) -> Result<()> {
    windows_launch(config)?.disable()?;
    Ok(())
}

#[cfg(windows)]
fn windows_launch(config: &Config) -> Result<auto_launch::AutoLaunch> {
    let args = start_args(config);
    Ok(auto_launch::AutoLaunchBuilder::new()
        .set_app_name(&label(config))
        .set_app_path(&std::env::current_exe()?.to_string_lossy())
        .set_args(&args.iter().map(|a| a.to_str().unwrap_or_default()).collect::<Vec<_>>())
        .build()?)
}

fn label(config: &Config) -> String {
    format!("dev.hunt{}", config.instance())
}

fn start_args(config: &Config) -> Vec<std::ffi::OsString> {
    vec!["--home".into(), config.home.clone().into(), "start".into(), "--quiet".into()]
}

/// Logs a failure instead of returning it.
pub async fn notify(title: String, body: String) {
    let shown =
        tokio::task::spawn_blocking(move || notify_rust::Notification::new().summary(&title).body(&body).show()).await;
    if let Ok(Err(err)) = shown {
        tracing::warn!("could not show a notification: {err}");
    }
}

/// The morning summary: what needs you, and what hunt found overnight.
pub async fn rundown(app: &Arc<App>) -> Result<()> {
    let today = insights::repo::today(&app.db).await?;
    let mut needs = vec![];
    if today.ready > 0 {
        needs.push(format!("{} to review", today.ready));
    }
    if today.replies > 0 {
        needs.push(format!("{} replies to check", today.replies));
    }
    if today.manual > 0 {
        needs.push(format!("{} to apply to yourself", today.manual));
    }
    let title = if needs.is_empty() { "Nothing needs you today".to_string() } else { needs.join(" · ") };
    let body = format!("{} new jobs found, {} shortlisted in the last day.", today.new_24h, today.shortlisted_24h);
    notify(title, body).await;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Backup;

impl TaskHandler for Backup {
    const IDENTIFIER: &'static str = "backup";

    async fn run(self, ctx: WorkerContext) -> impl IntoTaskHandlerResult {
        backup(&App::of(&ctx)).await
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Rundown;

impl TaskHandler for Rundown {
    const IDENTIFIER: &'static str = "rundown";

    async fn run(self, ctx: WorkerContext) -> impl IntoTaskHandlerResult {
        rundown(&App::of(&ctx)).await
    }
}
