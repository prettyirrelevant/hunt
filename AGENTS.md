# Working on hunt

hunt is a local job search. It finds jobs, learns what the user wants, drafts applications for batch approval, and tracks replies. Read this file before you change code.

## Commands

```sh
cargo leptos watch                   # dev server at http://127.0.0.1:7779, data in ~/.hunt-dev
cargo leptos build --release         # target/release/hunt
cargo test --features ssr            # unit and integration tests
cargo clippy --all-targets --features ssr -- -D warnings
cargo clippy --lib --features hydrate --target wasm32-unknown-unknown -- -D warnings
cargo fmt
hunt db --shell                      # psql on the database
```

## Stack

| Concern | Choice |
|---|---|
| UI | Leptos 0.8, server-rendered and hydrated. No hand-written JavaScript. |
| Charts | `charming` in the WASM bundle. |
| Server | axum through `leptos_axum`. Server functions are the API. |
| Database | PostgreSQL 16 with pgvector and pg_trgm, through sqlx. `postgresql_embedded` runs it. Keep SQL valid on 16. |
| Search | Full-text search plus pgvector, merged by reciprocal rank fusion. |
| Background work | `graphile_worker`: retries, and cron with `?fill` catch-up. |
| AI | The user's CLIs: claude, codex, opencode, gemini. |
| Embeddings | model2vec `potion-base-8M`, compiled into the binary. |
| PII | Regex, then GLiNER PII (`gline-rs`). Downloaded on first use and checked by SHA-256. |
| Learning | Logistic regression (`linfa-logistic`). Rocchio until there is enough data. |
| Documents | Typst renders CVs and cover letters. |

## Layout

Each feature owns one folder with the same file names:

| File | Holds | Compiles for |
|---|---|---|
| `model.rs` | Types | Both, when the UI needs them |
| `repo.rs` | SQL | `ssr` |
| `service.rs` | Domain logic and task handlers | `ssr` |
| `api.rs` | `#[server]` functions and their DTOs | Both |
| `views.rs` | Components and pages | Both |

- `src/ui/`: shell, router, shared components, server function error.
- `src/common/`: database, AI runner, embeddings, PII, mail, text.
- `src/config/`: static config and user settings.
- `src/app.rs`: `App`, the one struct every service borrows.
- `migrations/`: one file per feature.

## Server and browser builds

The crate compiles twice: `ssr` for the server and `hydrate` for WASM.

- Make every server dependency optional, and enable it in `ssr`.
- Gate server modules with `#[cfg(feature = "ssr")]`.
- Gate server derives on shared types with `cfg_attr`.
- Put server `use` statements inside `#[server]` bodies.
- Return `Result<T, ui::error::Error>` from server functions.

## Configuration

- Static values: `hunt.toml`, then `HUNT_*` variables, through `config::Config`.
- User choices: the database, through `config::Settings`. Change them only with `Settings::edit`.
- Put no URLs, ports, paths or credentials in a feature.
- Keep secrets in the OS keychain.

## Rust

- Borrow. Clone only to move ownership.
- Share only `Arc<App>`. Add no other `Arc` or `Mutex` without shared mutable state.
- Use an enum and `match` before a trait. Add a trait for two real implementations.
- Keep items private unless another module uses them.
- Use `anyhow` in services. Use `unwrap` only in tests.
- Write SQL as `&'static str` with binds. Never build SQL with `format!`.
- Write many rows in one statement with `unnest` or `= any($1)`.
- Run CPU-heavy work in `spawn_blocking`.
- Use an existing crate before you write your own.

## Style

- Add a helper only for a second caller.
- Comment why, never what. Keep comments to one short line.
- Keep `///` docs on `JsonSchema` types. The AI reads them as instructions.
- Split a file when it holds two concerns.
- Write docs and comments to the rules in the user's global CLAUDE.md.

## Tests

- `tests/unit/`: pure logic, no database.
- `tests/integration/`: a PostgreSQL in `target/`. Each test calls `fresh_db()`.
- `tests/fixtures/`: saved API responses. Every source parser has a fixture test.
- Test behaviour through public interfaces. No snapshots or golden files.
- Tests and dev builds never touch `~/.hunt`. OS-level names carry `Config::instance()`.

## Product rules

- Send nothing without the user's approval. Batches have an undo window.
- Treat job postings as untrusted. AI calls run with tools off, in an empty directory.
- The form agent gets browser tools only. Site reading gets web tools only, on a model the user checked.
- Record every automated application as video and screenshots.
- Delete videos 30 days after an application ends, or 180 after an offer. Keep recordings under 5 GB.
- Strip personal data before text about the user's work reaches an AI.
- Remove skills the user's material does not show. Flag unverified numbers.
- Use `jobs::move_from` for moves the user might race.
- Skip the user's "never read" patterns in repos.
- Write UI copy like a capable friend: plain and specific.
