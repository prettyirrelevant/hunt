use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use postgresql_embedded::{PostgreSQL, Settings, Status, VersionReq};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{io::AsyncWriteExt, process::Command};
use tracing::info;

use crate::config::{Config, Database};

/// Where the database runs, chosen by `database` in `hunt.toml`.
pub enum Server {
    /// Run by hunt from `postgres` in the hunt home. `owned` is set when this
    /// process started the server, and dropping it stops the server.
    Embedded { bin: PathBuf, url: String, owned: Option<Box<PostgreSQL>> },
    /// Your own server. Backups use `pg_dump` from your PATH.
    External(String),
}

pub async fn connect(config: &Config) -> Result<(PgPool, Server)> {
    let server = match &config.database {
        Database::Embedded => embedded(&config.home.join("postgres")).await?,
        Database::Url(url) => Server::External(url.clone()),
    };
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(server.url())
        .await
        .context("cannot reach the database")?;
    sqlx::migrate!().run(&pool).await.context("database migration failed")?;
    Ok((pool, server))
}

/// Downloads PostgreSQL and pgvector the first time, then starts the server,
/// or joins the one another hunt process already runs.
pub async fn embedded(dir: &Path) -> Result<Server> {
    let mut settings = Settings::new();
    // `Settings::new` creates two temporary folders that hunt does not use.
    let scratch =
        [settings.data_dir.clone(), settings.password_file.parent().map(Path::to_path_buf).unwrap_or_default()];
    for folder in scratch {
        tokio::fs::remove_dir(folder).await.ok();
    }
    // pgvector ships prebuilt for PostgreSQL 16 only.
    settings.version = VersionReq::parse("=16")?;
    settings.installation_dir = dir.join("install");
    settings.data_dir = dir.join("data");
    settings.password_file = dir.join("password");
    settings.temporary = false;
    if let Ok(password) = tokio::fs::read_to_string(&settings.password_file).await {
        settings.password = password;
    }
    tokio::fs::create_dir_all(dir).await?;

    let mut postgres = PostgreSQL::new(settings);
    if postgres.status() == Status::NotInstalled {
        info!("downloading PostgreSQL once, about 40 MB");
    }
    postgres.setup().await.context("could not install PostgreSQL")?;
    let installed = postgresql_extensions::get_installed_extensions(postgres.settings()).await?;
    if !installed.iter().any(|e| e.name() == "pgvector_compiled") {
        postgresql_extensions::install(postgres.settings(), "portal-corp", "pgvector_compiled", &VersionReq::STAR)
            .await
            .context("no pgvector build exists for this computer. Set database in hunt.toml to your own PostgreSQL with pgvector")?;
    }
    let bin = postgres.settings().binary_dir();

    // A pid file can remain after a restart, so the server must also answer.
    if postgres.status() == Status::Started
        && let Some(port) = running_port(&postgres.settings().data_dir).await
    {
        let mut settings = postgres.settings().clone();
        settings.port = port;
        let url = settings.url("hunt");
        if PgPoolOptions::new().connect(&url).await.is_ok() {
            // `forget` keeps Drop from stopping a server this process did not start.
            std::mem::forget(postgres);
            return Ok(Server::Embedded { bin, url, owned: None });
        }
    }
    postgres.start().await.context("could not start PostgreSQL")?;
    if !postgres.database_exists("hunt").await? {
        postgres.create_database("hunt").await?;
    }
    let url = postgres.settings().url("hunt");
    Ok(Server::Embedded { bin, url, owned: Some(Box::new(postgres)) })
}

/// PostgreSQL writes its port on the fourth line of `postmaster.pid`.
async fn running_port(data: &Path) -> Option<u16> {
    let pid = tokio::fs::read_to_string(data.join("postmaster.pid")).await.ok()?;
    pid.lines().nth(3)?.trim().parse().ok()
}

impl Server {
    pub async fn backup(&self, dir: &Path) -> Result<PathBuf> {
        tokio::fs::create_dir_all(dir).await?;
        let path = dir.join(format!("hunt-{}.dump", chrono::Local::now().format("%Y-%m-%d")));
        let output = self.tool("pg_dump", &["-Fc"]).output().await.context("pg_dump is not installed")?;
        if !output.status.success() {
            bail!("pg_dump failed: {}", String::from_utf8_lossy(&output.stderr).trim());
        }
        tokio::fs::write(&path, output.stdout).await?;
        Ok(path)
    }

    pub async fn restore(&self, file: &Path) -> Result<()> {
        let dump = tokio::fs::read(file).await?;
        let mut child = self
            .tool("pg_restore", &["--clean", "--if-exists", "--no-owner"])
            .stdin(Stdio::piped())
            .spawn()
            .context("pg_restore is not installed")?;
        let mut stdin = child.stdin.take().expect("stdin is piped");
        stdin.write_all(&dump).await?;
        drop(stdin);
        if !child.wait().await?.success() {
            bail!("pg_restore failed");
        }
        Ok(())
    }

    pub async fn shell(&self) -> Result<()> {
        self.tool("psql", &[]).status().await.context("psql is not installed")?;
        Ok(())
    }

    pub fn url(&self) -> &str {
        match self {
            Server::Embedded { url, .. } | Server::External(url) => url,
        }
    }

    /// Whether this process started the server, and so stops it on exit.
    pub fn started_here(&self) -> bool {
        matches!(self, Server::Embedded { owned: Some(_), .. })
    }

    /// `pg_dump`, `pg_restore` or `psql`, from the same PostgreSQL as the server.
    fn tool(&self, name: &str, args: &[&str]) -> Command {
        let mut command = match self {
            Server::Embedded { bin, .. } => Command::new(bin.join(name)),
            Server::External(_) => Command::new(name),
        };
        command.args(args).arg("--dbname").arg(self.url());
        command
    }
}
