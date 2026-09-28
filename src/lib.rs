#![recursion_limit = "256"]

pub mod insights;
pub mod jobs;
pub mod matching;
pub mod profile;
pub mod review;
pub mod settings;
pub mod tracking;
pub mod ui;

#[cfg(feature = "ssr")]
pub mod app;
#[cfg(feature = "ssr")]
pub mod applying;
#[cfg(feature = "ssr")]
pub mod common;
#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod discovery;
#[cfg(feature = "ssr")]
pub mod server;
#[cfg(feature = "ssr")]
pub mod system;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(ui::shell::App);
}
