use std::{
    collections::HashMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use ::config::{Config as Sources, Environment, File};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Static settings: defaults, then `~/.hunt/hunt.toml`, then `HUNT_*`
/// environment variables. `DATABASE_URL` also sets `database`, for sqlx tooling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub database: Database,
    pub port: u16,
    pub log: String,
    #[serde(skip)]
    pub home: PathBuf,
}

/// `embedded` runs PostgreSQL inside hunt. A `postgres://` URL uses your own
/// server, which needs the `vector` and `pg_trgm` extensions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Database {
    Embedded,
    Url(String),
}

impl From<String> for Database {
    fn from(value: String) -> Database {
        match value.as_str() {
            "embedded" => Database::Embedded,
            _ => Database::Url(value),
        }
    }
}

impl From<Database> for String {
    fn from(database: Database) -> String {
        match database {
            Database::Embedded => "embedded".into(),
            Database::Url(url) => url,
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config { database: Database::Embedded, port: 7777, log: "hunt=info,warn".into(), home: PathBuf::new() }
    }
}

impl Config {
    /// `home` comes from `--home`, then `HUNT_HOME`, then `~/.hunt`.
    pub fn load(home: Option<PathBuf>) -> Result<Config> {
        let home = home.or_else(|| std::env::var_os("HUNT_HOME").map(PathBuf::from)).unwrap_or_else(default_home);
        std::fs::create_dir_all(&home).with_context(|| format!("cannot create {}", home.display()))?;

        let database_url: HashMap<String, String> =
            std::env::var("DATABASE_URL").ok().map(|url| ("DATABASE".into(), url)).into_iter().collect();
        let mut config: Config = Sources::builder()
            .add_source(Sources::try_from(&Config::default())?)
            .add_source(File::from(home.join("hunt.toml")).required(false))
            .add_source(Environment::default().source(Some(database_url)))
            .add_source(Environment::with_prefix("HUNT").try_parsing(true))
            .build()
            .and_then(Sources::try_deserialize)
            .context("invalid hunt.toml or HUNT_ environment variable")?;
        config.home = home;
        Ok(config)
    }

    /// Names shared with the operating system, such as the Keychain entry and
    /// the login item, end with this. A second hunt home, for tests or
    /// development, so never touches the live one. Empty for `~/.hunt`.
    pub fn instance(&self) -> String {
        if self.home == default_home() {
            return String::new();
        }
        let hash = Sha256::digest(self.home.to_string_lossy().as_bytes());
        hash.iter().take(4).fold("-".to_string(), |mut name, byte| {
            let _ = write!(name, "{byte:02x}");
            name
        })
    }

    pub fn backups(&self, chosen: Option<&Path>) -> PathBuf {
        chosen.map_or_else(|| self.home.join("backups"), Path::to_path_buf)
    }

    pub fn logs(&self) -> PathBuf {
        self.home.join("logs")
    }

    pub fn documents(&self) -> PathBuf {
        self.home.join("documents")
    }
}

fn default_home() -> PathBuf {
    directories::BaseDirs::new().expect("a home directory").home_dir().join(".hunt")
}
