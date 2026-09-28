//! Runs against a PostgreSQL in `target/`, with a fresh database per test.

use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use hunt::common::db::{self, Server};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::OnceCell;

mod live;
mod profile;
mod record;

static SERVER: OnceCell<PgConnectOptions> = OnceCell::const_new();
static NEXT: AtomicUsize = AtomicUsize::new(0);

pub async fn fresh_db() -> PgPool {
    let server = SERVER.get_or_init(start).await;
    let name = format!("test_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
    let admin = PgPoolOptions::new().max_connections(1).connect_with(server.clone()).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("create database {name}"))).execute(&admin).await.unwrap();
    let pool = PgPoolOptions::new().connect_with(server.clone().database(&name)).await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    pool
}

async fn start() -> PgConnectOptions {
    let server = db::embedded(&Path::new(env!("CARGO_TARGET_TMPDIR")).join("postgres")).await.unwrap();
    let options: PgConnectOptions = server.url().parse().unwrap();
    if let Server::Embedded { owned: Some(postgres), .. } = server {
        // Left running, so the next run starts in seconds.
        std::mem::forget(postgres);
    }
    let admin = PgPoolOptions::new().max_connections(1).connect_with(options.clone()).await.unwrap();
    let old: Vec<String> = sqlx::query_scalar("select datname from pg_database where datname like 'test\\_%'")
        .fetch_all(&admin)
        .await
        .unwrap();
    for name in old {
        sqlx::query(sqlx::AssertSqlSafe(format!("drop database {name} with (force)"))).execute(&admin).await.unwrap();
    }
    options
}
