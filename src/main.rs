use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use clap::{Parser, Subcommand};
use graphile_worker::WorkerOptions;
use hunt::{
    app::{App, QUEUE_SCHEMA},
    applying::service::{Apply, Prune},
    config::Config,
    discovery::service::Sweep,
    matching::service::{Learn, Score},
    profile::service::{ReadRepos, ReadSites, Rebuild, Summarize},
    review::service::Draft,
    server,
    system::service::{self as system, Backup, Rundown},
    tracking::service::CheckInbox,
};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

// UTC. `fill` reruns anything missed while the laptop slept or was off.
const SCHEDULE: &str = "
0 */3 * * * sweep ?fill=6h&queue=sweep
*/15 * * * * check_inbox ?fill=1h&queue=inbox
0 7 * * 1 read_repos ?fill=7d
0 8 * * 1 read_sites ?fill=7d
0 3 * * * learn ?fill=2d
30 2 * * * backup ?fill=2d
0 7 * * * rundown
0 4 * * * prune_recordings ?fill=2d
";

#[derive(Parser)]
#[command(version, about = "A job search that runs on your machine")]
struct Cli {
    /// Where hunt keeps its data. Defaults to `HUNT_HOME`, then `~/.hunt` (`~/.hunt-dev` in debug builds).
    #[arg(long, global = true)]
    home: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run hunt and open the dashboard. This is the default.
    Start {
        /// Do not open the browser.
        #[arg(long)]
        quiet: bool,
    },
    /// Search every source now.
    Sweep,
    /// Save a database backup now.
    Backup,
    /// Replace the database with a backup file.
    Restore { file: PathBuf },
    /// Print the database URL, or open psql on it with --shell.
    Db {
        #[arg(long)]
        shell: bool,
    },
    /// Start hunt at login and keep it running.
    Install,
    /// Stop starting hunt at login.
    Uninstall,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(cli.home)?;
    let _log = logging(&config);

    match cli.command.unwrap_or(Command::Start { quiet: false }) {
        Command::Start { quiet } => start(config, quiet).await,
        Command::Sweep => hunt::discovery::service::sweep(&App::start(config).await?).await,
        Command::Backup => system::backup(&App::start(config).await?).await,
        Command::Restore { file } => {
            let (pool, server) = hunt::common::db::connect(&config).await?;
            pool.close().await;
            server.restore(&file).await
        }
        Command::Db { shell } => database(&config, shell).await,
        Command::Install => system::install(&config),
        Command::Uninstall => system::uninstall(&config),
    }
}

async fn start(config: Config, quiet: bool) -> Result<()> {
    let port = config.port;
    let app = App::start(config).await?;

    let worker = WorkerOptions::default()
        .pg_pool(app.db.clone())
        .schema(QUEUE_SCHEMA)
        .concurrency(3)
        .define_job::<Sweep>()
        .define_job::<Score>()
        .define_job::<Draft>()
        .define_job::<ReadRepos>()
        .define_job::<ReadSites>()
        .define_job::<Summarize>()
        .define_job::<Rebuild>()
        .define_job::<Backup>()
        .define_job::<Rundown>()
        .define_job::<Apply>()
        .define_job::<CheckInbox>()
        .define_job::<Learn>()
        .define_job::<Prune>()
        .add_extension(Arc::clone(&app))
        .with_cron(SCHEDULE)?
        .init()
        .await?;
    let read_before: bool =
        sqlx::query_scalar("select exists (select 1 from notes where source = 'repo')").fetch_one(&app.db).await?;
    if !read_before {
        app.queue(ReadRepos, "read_repos").await?;
    }
    let warm = Arc::clone(&app);
    tokio::spawn(async move {
        if let Err(err) = warm.redactor().await {
            tracing::warn!("{err:#}. hunt tries again when it next needs it.");
        }
    });
    tokio::spawn(async move {
        if let Err(err) = worker.run().await {
            tracing::error!("background worker stopped: {err}");
        }
    });

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let url = format!("http://localhost:{port}");
    info!("hunt is running at {url}");
    if !quiet {
        open::that(&url).ok();
    }
    axum::serve(listener, server::router(app)).with_graceful_shutdown(stopped()).await?;
    info!("hunt stopped");
    Ok(())
}

async fn database(config: &Config, shell: bool) -> Result<()> {
    let (pool, server) = hunt::common::db::connect(config).await?;
    pool.close().await;
    if shell {
        return server.shell().await;
    }
    println!("{}", server.url());
    if server.started_here() {
        eprintln!("hunt was not running, so it started the database for you. It stops when you press Ctrl-C.");
        stopped().await;
    }
    Ok(())
}

async fn stopped() {
    #[cfg(unix)]
    {
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await.ok();
    info!("stopping");
}

fn logging(config: &Config) -> tracing_appender::non_blocking::WorkerGuard {
    let (file, guard) = tracing_appender::non_blocking(tracing_appender::rolling::daily(config.logs(), "hunt.log"));
    tracing_subscriber::registry()
        .with(EnvFilter::new(&config.log))
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().with_ansi(false).with_writer(file))
        .init();
    guard
}
