# hunt

A job search that runs on your machine. hunt reads 20+ job sources, drops what cannot work for you, learns what you like, and drafts tailored applications. You approve them in batches, and it tracks every reply.

## Install

Download the archive for your computer from [Releases](https://github.com/prettyirrelevant/hunt/releases), unpack it, and put `hunt` on your PATH. Then:

```sh
hunt            # opens the dashboard at http://localhost:7777
hunt install    # starts hunt at login and keeps it running
```

hunt also needs:

- At least one AI CLI, logged in: `claude`, `codex`, `opencode` or `gemini`
- Node, for the browser that fills application forms, and its Chromium: `npx -y playwright install chromium`

hunt runs its own PostgreSQL. The first run downloads PostgreSQL (about 40 MB) and the 155 MB model that removes personal data, once. Your data lives in `~/.hunt`.

pgvector ships prebuilt for macOS on Apple silicon, Linux x86_64 and Windows x86_64. On other computers, set `database` in `~/.hunt/hunt.toml` to your own PostgreSQL with pgvector.

Other commands: `hunt sweep` searches now, `hunt backup` saves a backup, `hunt restore <file>` restores one, `hunt db` prints the database URL, `hunt db --shell` opens psql on it, and `hunt uninstall` stops the login item. `--home <dir>` runs a separate hunt with its own data.

## Build from source

You need Rust 1.94 or later, `rustup target add wasm32-unknown-unknown`, and `cargo install cargo-leptos --locked`.

```sh
cargo leptos build --release
cp target/release/hunt ~/.local/bin/
```

## First steps

1. Settings: choose your country, add your contact details, and connect Gmail with an app password.
2. You: upload your CV, add your site or blog, and say anything your CV does not. hunt also reads the git repositories in your code folders.
3. Wait for the first search, then approve or skip applications in Review.

## Develop

```sh
cargo leptos watch                     # http://127.0.0.1:7779 with hot reload
cargo test --features ssr              # unit and integration tests; the first run downloads PostgreSQL into target/
cargo test --features ssr --test integration live -- --ignored --nocapture   # checks every live job source
```

Debug builds keep their data in `~/.hunt-dev` and serve on port 7779, so development never touches the app you use. Release builds use `~/.hunt` and port 7777.

Push a tag such as `v0.1.0` to build release binaries for macOS, Linux and Windows.

## Configure

Everything has a default. To change one, copy `hunt.example.toml` to `~/.hunt/hunt.toml` and edit it. You choose everything else in the dashboard.

## License

MIT. See `LICENSE`. The Hanken Grotesk and JetBrains Mono fonts in `assets/public/fonts` are under the SIL Open Font License 1.1, with their texts beside them.
