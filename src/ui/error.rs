use std::fmt;

use leptos::server_fn::{
    codec::JsonEncoding,
    error::{FromServerFnError, ServerFnErrorErr},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Error(pub String);

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromServerFnError for Error {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        Error(value.to_string())
    }
}

#[cfg(feature = "ssr")]
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("{err:#}");
        Error(format!("{err:#}"))
    }
}

#[cfg(feature = "ssr")]
impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        anyhow::Error::from(err).into()
    }
}
