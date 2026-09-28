use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgExecutor, PgPool};

use crate::common::ai::Provider;
use crate::discovery::model::Reach;
use crate::discovery::sources::ats::Board;

/// Added to documents locally. Never sent to an AI.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Contact {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub links: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub reach: Option<Reach>,
    pub contact: Contact,
    pub providers: Vec<Provider>,
    /// The app password lives in the macOS Keychain.
    pub email: Option<String>,
    pub backup_dir: Option<PathBuf>,
    pub watched: Vec<Board>,
    pub repo_roots: Vec<PathBuf>,
    pub ignore: Vec<String>,
    pub sites: Vec<String>,
    pub web_models: HashMap<Provider, String>,
}

impl Default for Settings {
    fn default() -> Settings {
        let home = directories::BaseDirs::new().expect("a home directory").home_dir().to_path_buf();
        Settings {
            reach: None,
            contact: Contact::default(),
            providers: Provider::ALL.into_iter().filter(|p| p.installed()).collect(),
            email: None,
            backup_dir: None,
            watched: vec![],
            repo_roots: ["Developer", "Projects", "projects", "code", "src", "dev", "work", "repos"]
                .iter()
                .map(|dir| home.join(dir))
                .filter(|dir| dir.is_dir())
                .collect(),
            ignore: vec![],
            sites: vec![],
            web_models: HashMap::from([(Provider::Claude, "haiku".into()), (Provider::Codex, "gpt-5.6-luna".into())]),
        }
    }
}

impl Settings {
    pub async fn load(db: impl PgExecutor<'_>) -> Result<Settings> {
        let rows: Vec<(String, Value)> = sqlx::query_as("select key, value from settings").fetch_all(db).await?;
        Ok(serde_json::from_value(Value::Object(rows.into_iter().collect::<Map<_, _>>()))?)
    }

    pub async fn edit<T>(db: &PgPool, change: impl FnOnce(&mut Settings) -> T) -> Result<T> {
        let mut tx = db.begin().await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext('settings'))").execute(&mut *tx).await?;
        let mut settings = Settings::load(&mut *tx).await?;
        let out = change(&mut settings);
        let Value::Object(fields) = serde_json::to_value(&settings)? else { unreachable!("Settings is a struct") };
        let (keys, values): (Vec<String>, Vec<Value>) = fields.into_iter().unzip();
        sqlx::query(
            "insert into settings (key, value) select * from unnest($1::text[], $2::jsonb[])
             on conflict (key) do update set value = excluded.value",
        )
        .bind(&keys)
        .bind(&values)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(out)
    }
}
