# hunt

A job search that runs on your machine. hunt reads 20+ job sources, learns what you like, and drafts applications. You approve them in batches. hunt tracks every reply.

## Install

1. Download the archive for your computer from [Releases](https://github.com/prettyirrelevant/hunt/releases).
2. Put `hunt` on your PATH.
3. Log in to one AI CLI: `claude`, `codex`, `opencode` or `gemini`.
4. Install Chromium for form filling: `npx -y playwright install chromium`.
5. Run it:

```sh
hunt            # dashboard at http://localhost:7777
hunt install    # start at login
```

The first run downloads PostgreSQL (40 MB) and a PII model (155 MB). Your data lives in `~/.hunt`.

Prebuilt pgvector exists for macOS on Apple silicon, Linux x86_64 and Windows x86_64. On other computers, set `database` in `~/.hunt/hunt.toml` to a PostgreSQL with pgvector.

## Commands

| Command | Does |
|---|---|
| `hunt` | Starts hunt and opens the dashboard. |
| `hunt sweep` | Searches every source now. |
| `hunt backup` | Saves a database backup. |
| `hunt restore <file>` | Restores a backup. |
| `hunt db [--shell]` | Prints the database URL, or opens psql. |
| `hunt install` / `uninstall` | Adds or removes the login item. |

`--home <dir>` runs a separate hunt with its own data.

## First steps

1. In Settings, choose your country, add contact details, and connect Gmail with an app password.
2. In You, upload your CV and add your site. hunt also reads the git repos in your code folders.
3. After the first search, approve or skip applications in Review.

## Build

You need rustup and `cargo install cargo-leptos --locked`. rustup installs the Rust version in `rust-toolchain.toml`.

```sh
cargo leptos build --release     # target/release/hunt
cargo leptos watch               # dev server at http://127.0.0.1:7779
cargo test --features ssr        # first run downloads PostgreSQL into target/
```

Debug builds use `~/.hunt-dev` and port 7779. Release builds use `~/.hunt` and port 7777.

Push a tag such as `v0.1.0` to publish binaries for macOS, Linux and Windows.

## Configure

Everything has a default. To change one, copy `hunt.example.toml` to `~/.hunt/hunt.toml`.

## License

MIT. The Hanken Grotesk and JetBrains Mono fonts use the SIL Open Font License 1.1. Their license texts are in `assets/public/fonts`.
