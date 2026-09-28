use std::{fmt, sync::Arc, time::Duration};

use tokio::sync::OnceCell;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use graphile_worker::{JobSpec, TaskHandler, WorkerContext, WorkerUtils};
use sqlx::PgPool;

use crate::common::{ai::Ai, db, embed::Embedder, pii::Redactor};
use crate::config::{Config, Settings};

pub const QUEUE_SCHEMA: &str = "graphile_worker";

pub struct App {
    pub config: Config,
    pub db: PgPool,
    pub server: db::Server,
    pub http: reqwest::Client,
    pub ai: Ai,
    pub embedder: Embedder,
    redactor: OnceCell<Redactor>,
    worker: WorkerUtils,
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("App")
    }
}

impl App {
    pub async fn start(config: Config) -> Result<Arc<App>> {
        let (db, server) = db::connect(&config).await?;
        let worker = WorkerUtils::new(db.clone(), QUEUE_SCHEMA);
        worker.migrate().await?;
        let http = reqwest::Client::builder()
            .user_agent(concat!("hunt/", env!("CARGO_PKG_VERSION"), " (personal job search)"))
            .timeout(Duration::from_secs(30))
            .build()?;
        let ai = Ai::new(config.home.join("ai"))?;
        let embedder = tokio::task::spawn_blocking(Embedder::load).await??;
        Ok(Arc::new(App { config, db, server, http, ai, embedder, redactor: OnceCell::new(), worker }))
    }

    /// Read on every call, so dashboard changes apply at once.
    pub async fn settings(&self) -> Result<Settings> {
        Settings::load(&self.db).await
    }

    /// A waiting task with the same `key` is replaced.
    pub async fn queue<T: TaskHandler>(&self, task: T, key: impl Into<String>) -> Result<()> {
        let spec = JobSpec { job_key: Some(key.into()), ..Default::default() };
        self.worker.add_job(task, spec).await?;
        Ok(())
    }

    /// Adds many tasks of one kind in one round trip, each with its key.
    pub async fn queue_all<T: TaskHandler + Clone>(
        &self,
        tasks: Vec<(T, String)>,
        at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let specs: Vec<JobSpec> = tasks
            .iter()
            .map(|(_, key)| JobSpec { job_key: Some(key.clone()), run_at: at, ..Default::default() })
            .collect();
        let jobs: Vec<(T, &JobSpec)> = tasks.into_iter().map(|(task, _)| task).zip(&specs).collect();
        self.worker.add_jobs(&jobs).await?;
        Ok(())
    }

    pub fn of(ctx: &WorkerContext) -> Arc<App> {
        ctx.get_ext::<Arc<App>>().expect("the worker is built with the app").clone()
    }

    /// Loads the PII model, downloading it the first time.
    pub async fn redactor(&self) -> Result<&Redactor> {
        let dir = self.config.home.join("models/pii");
        self.redactor
            .get_or_try_init(|| Redactor::fetch(&self.http, &dir))
            .await
            .context("the PII model is not ready, so nothing about your work goes to an AI yet")
    }

    pub async fn redact(self: &Arc<App>, text: String) -> Result<String> {
        self.redactor().await?;
        let app = Arc::clone(self);
        tokio::task::spawn_blocking(move || app.redactor.get().expect("loaded above").redact(&text)).await?
    }
}
