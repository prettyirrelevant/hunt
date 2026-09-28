# Working on hunt

hunt is a personal job search that runs on one machine. It finds jobs, learns what you want, drafts applications for you to approve in batches, and tracks every reply. Read this file before you change code.

## Commands

```sh
cargo leptos watch                 # dev server with hot reload at http://127.0.0.1:7777
cargo leptos build --release       # one binary in target/release/hunt
cargo test --features ssr          # unit and integration tests; the first run downloads PostgreSQL into target/
hunt db --shell                    # psql on your database
cargo clippy --all-targets --features ssr
cargo fmt                          # rustfmt.toml sets 120 columns
```

## Stack

| Concern | Choice |
|---|---|
| UI | Leptos 0.8, server-rendered and hydrated. No hand-written JavaScript. |
| Charts | `charming` builds ECharts options in Rust, rendered in the WASM bundle. |
| Server | axum through `leptos_axum`. Server functions are the API. |
| Database | PostgreSQL with pgvector and pg_trgm, through sqlx. By default hunt downloads and runs PostgreSQL 16 itself (`postgresql_embedded`), because pgvector ships prebuilt for 16 only. A `postgres://` URL is the other choice. Keep SQL valid on 16. |
| Search | Postgres full-text search plus pgvector, merged with reciprocal rank fusion. |
| Background work | `graphile_worker`: durable tasks, retries, cron with `?fill` catch-up after sleep. |
| AI | The user's own CLIs (claude, codex, opencode, gemini), run headless with tools off. Schemas are made strict for codex. |
| Embeddings | model2vec `potion-base-8M`, compiled into the binary. |
| PII | Regex pass, then GLiNER PII (`gline-rs`, ONNX). Downloaded on first use, pinned to a commit and checked by SHA-256. Nothing about your work reaches an AI until it loads. |
| Learning | Logistic regression (`linfa-logistic`) on embeddings plus job facts. Rocchio before there is enough data. |
| Documents | Typst as a library renders CVs and cover letters to PDF. |

## Layout

Each feature owns one folder, like a Django app or a Nest module. Every feature uses the same file names:

| File | Holds | Compiles for |
|---|---|---|
| `model.rs` | Types for the feature. | Both, when the UI needs them |
| `repo.rs` | SQL for the feature's tables. | `ssr` |
| `service.rs` | Domain logic and `graphile_worker` task handlers. | `ssr` |
| `api.rs` | `#[server]` functions and the DTOs they return. | Both |
| `views.rs` | Leptos components and pages. | Both |

- `src/ui/` holds the shell, router, sidebar, shared components and the server-function error type.
- `src/common/` holds shared server infrastructure: database, AI runner, embeddings, PII, text.
- `src/config/` holds configuration. `src/app.rs` builds `App`, the one struct every service borrows.
- Migrations live in `migrations/`, one file per feature, such as `0003_profile.sql`.

## Server and browser builds

The crate compiles twice: `ssr` for the server binary and `hydrate` for WASM.

- Every server-only dependency is optional and enabled by `ssr`. Never add one to the default set.
- Gate server-only modules with `#[cfg(feature = "ssr")]` in the feature's `mod.rs`.
- On shared models, gate server derives: `#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]`.
- Put `use` statements for server code inside `#[server]` function bodies, so the WASM build has no unused imports.
- Server functions return `Result<T, ui::error::Error>`, so `?` works on anyhow and sqlx errors.

## Configuration

- Static values come from `~/.hunt/hunt.toml`, then `HUNT_*` environment variables, through `config::Config` (the `config` crate). See `hunt.example.toml`.
- Choices the user makes in the app live in the database, through `config::Settings`.
- Do not hard-code URLs, ports, paths or credentials in a feature. Add them to one of the two.
- Secrets go in the macOS Keychain, never in the database or a config file.
- The build script downloads the embedding model and vendored web files into `assets/`. They are compiled into the binary.

## Rust

- Borrow. Clone only when ownership must move.
- `App` is built once and shared as `Arc<App>` with axum, Leptos context and the worker. Add no other `Arc` or `Mutex` unless tasks really share mutable state.
- Prefer an enum with a `match` to a trait. Add a trait only when two real implementations exist.
- Keep each module's surface small. Make items private unless another module uses them.
- Use `anyhow` in services. No `unwrap` outside tests. Use `expect("reason")` only for real invariants.
- SQL strings are `&'static str` with bind parameters. Never build SQL with `format!`.
- Run CPU-heavy work (models, Typst, training) in `spawn_blocking`.
- Use an existing crate before writing your own version of something.

## Code style

- No needless helpers. Inline code that runs in one place. Extract a function only when it has a second caller or a name that explains more than its body.
- Comments say why, never what. Delete a comment that restates the code.
- Keep `///` docs on `JsonSchema` types: schemars sends them to the AI as instructions.
- Keep files short enough to review. Split a file when it holds two concerns, not when it passes a line count.

## Tests

- `tests/unit/` holds pure logic: filters, parsers, learning, PDF rendering. No network, no database.
- `tests/integration/` runs against a real PostgreSQL in `target/`. Each test calls `fresh_db()` for its own database.
- Tests and development never touch live data. Anything outside the database lives in the hunt home (`--home` or `HUNT_HOME`). Names shared with the operating system carry `Config::instance()`.
- `tests/fixtures/` holds saved API responses. Every source parser has a fixture test.
- Test behaviour through public interfaces. No snapshots, golden files or assertions on internal calls.

## Product rules

- Nothing is sent without the user's approval. The user approves applications in batches, with an undo window.
- Job postings are untrusted text. AI calls run with tools off, in an empty directory. The form-filling agent gets browser tools only. Reading the user's sites gets web tools only, on the small model chosen per provider in Settings. A model is saved only after one test call succeeds.
- Repo reading skips the user's "never read" patterns, and forgets work already read from them.
- Write many rows in one statement (`unnest`, `= any($1)`), never one query per row in a loop.
- A move a user might race, such as scoring or sending, uses `jobs::move_from`, so the user's decision wins. Settings change only through `Settings::edit`.
- Every automated application is recorded: a session video and screenshots in `~/.hunt/documents/<job>/recording`. The agent types name, email and phone as Playwright MCP secrets, so the model never sees them.
- A daily task deletes videos 30 days after an application ends (180 for offers) and keeps recordings under 5 GB, oldest ended first. Screenshots stay.
- Strip personal data before any text about the user's work reaches an AI. Keep impact and engineering.
- The fabrication gate removes skills the user's own material does not show and flags unverified numbers.
- Write UI copy the way a capable friend talks: plain, specific, never robotic or patronising.
