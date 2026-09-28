//! Every CLI runs in an empty directory with its tools off, except as each caller says.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::{io::AsyncWriteExt, process::Command};
use tracing::warn;

const TIMEOUT: Duration = Duration::from_mins(4);
const BROWSE_TIMEOUT: Duration = Duration::from_mins(10);
const PLAYWRIGHT_MCP: &str = "@playwright/mcp@0.0.82";
const WEB_TOOLS: &str = "WebFetch,WebSearch";
const CLAUDE_ALIASES: [&str; 4] = ["haiku", "sonnet", "opus", "fable"];
const USAGE_LIMIT_REST: Duration = Duration::from_mins(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Codex,
    Opencode,
    Gemini,
}

impl Provider {
    pub const ALL: [Provider; 4] = [Provider::Claude, Provider::Codex, Provider::Opencode, Provider::Gemini];

    pub fn name(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
            Provider::Opencode => "opencode",
            Provider::Gemini => "gemini",
        }
    }

    pub fn installed(self) -> bool {
        which::which(self.name()).is_ok()
    }
}

pub enum Status {
    Ready,
    Resting { minutes_left: u64 },
    Missing,
}

pub struct Answer<T> {
    pub value: T,
    pub provider: Provider,
}

pub struct Ai {
    workdir: PathBuf,
    resting: Mutex<HashMap<Provider, Instant>>,
}

enum Tools<'a> {
    Off,
    Web,
    Browser { mcp: &'a Path },
}

enum Failure {
    Unavailable(String),
    Failed(anyhow::Error),
}

impl Ai {
    pub fn new(workdir: PathBuf) -> Result<Ai> {
        std::fs::create_dir_all(&workdir)?;
        Ok(Ai { workdir, resting: Mutex::default() })
    }

    pub async fn models(&self, provider: Provider) -> Vec<String> {
        let listed = match provider {
            Provider::Claude => return CLAUDE_ALIASES.map(String::from).to_vec(),
            Provider::Gemini => return vec![],
            Provider::Codex => Command::new("codex").args(["debug", "models"]).output().await,
            Provider::Opencode => Command::new("opencode").arg("models").output().await,
        };
        let Ok(output) = listed else { return vec![] };
        let text = String::from_utf8_lossy(&output.stdout);
        match provider {
            Provider::Codex => {
                let catalog: Value = serde_json::from_str(&text).unwrap_or_default();
                catalog["models"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|m| m["visibility"] == "list")
                    .filter_map(|m| m["slug"].as_str().map(String::from))
                    .collect()
            }
            _ => text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect(),
        }
    }

    pub async fn check(&self, provider: Provider, model: &str) -> Result<()> {
        #[derive(Deserialize, JsonSchema)]
        struct Ping {
            ok: bool,
        }
        let schema = schema::<Ping>()?;
        match self.run(provider, "Reply with ok set to true.", &schema, &Tools::Off, Some(model)).await {
            Ok(value) => match serde_json::from_value(value) {
                Ok(Ping { ok: true }) => Ok(()),
                _ => bail!("it answered, but not as asked"),
            },
            Err(Failure::Unavailable(why)) => bail!("{why}"),
            Err(Failure::Failed(err)) => Err(err),
        }
    }

    pub fn status(&self, provider: Provider) -> Status {
        if !provider.installed() {
            return Status::Missing;
        }
        match self.resting.lock().expect("lock").get(&provider) {
            Some(until) if *until > Instant::now() => {
                Status::Resting { minutes_left: until.duration_since(Instant::now()).as_secs() / 60 + 1 }
            }
            _ => Status::Ready,
        }
    }

    pub async fn ask<T: DeserializeOwned + JsonSchema>(&self, order: &[Provider], prompt: &str) -> Result<Answer<T>> {
        self.answer(order, prompt, &Tools::Off, &HashMap::new()).await
    }

    pub async fn read_web<T: DeserializeOwned + JsonSchema>(
        &self,
        order: &[Provider],
        models: &HashMap<Provider, String>,
        prompt: &str,
    ) -> Result<Answer<T>> {
        self.answer(order, prompt, &Tools::Web, models).await
    }

    async fn answer<T: DeserializeOwned + JsonSchema>(
        &self,
        order: &[Provider],
        prompt: &str,
        tools: &Tools<'_>,
        models: &HashMap<Provider, String>,
    ) -> Result<Answer<T>> {
        let schema = schema::<T>()?;
        let mut tried = vec![];
        for &provider in order {
            if !matches!(self.status(provider), Status::Ready) {
                continue;
            }
            match self.run(provider, prompt, &schema, tools, models.get(&provider).map(String::as_str)).await {
                Ok(value) => {
                    let value = serde_json::from_value(value)
                        .with_context(|| format!("{} answered in the wrong shape", provider.name()))?;
                    return Ok(Answer { value, provider });
                }
                Err(Failure::Unavailable(why)) => {
                    warn!("{} is unavailable: {why}", provider.name());
                    self.resting.lock().expect("lock").insert(provider, Instant::now() + USAGE_LIMIT_REST);
                    tried.push(format!("{}: {why}", provider.name()));
                }
                Err(Failure::Failed(err)) => {
                    return Err(err.context(format!("{} failed", provider.name())));
                }
            }
        }
        if tried.is_empty() {
            bail!("no AI provider is ready");
        }
        bail!("every AI provider is unavailable ({})", tried.join("; "))
    }

    /// claude with Playwright tools only, recorded to `recording`. `secrets` are typed by name.
    pub async fn browse<T: DeserializeOwned + JsonSchema>(
        &self,
        prompt: &str,
        recording: &Path,
        secrets: &[(&str, &str)],
    ) -> Result<T> {
        tokio::fs::create_dir_all(recording).await?;
        let playwright = recording.join("playwright.json");
        let mcp = recording.join("mcp.json");
        let secrets: serde_json::Map<String, Value> =
            secrets.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (k.to_string(), Value::from(*v))).collect();
        tokio::fs::write(
            &playwright,
            serde_json::json!({
                "browser": {
                    "browserName": "chromium",
                    "isolated": true,
                    "launchOptions": { "headless": true },
                    "contextOptions": {
                        "viewport": { "width": 1280, "height": 800 },
                        "recordVideo": { "dir": recording, "size": { "width": 1280, "height": 800 } },
                    },
                },
                "outputDir": recording,
                "secrets": secrets,
            })
            .to_string(),
        )
        .await?;
        tokio::fs::write(
            &mcp,
            serde_json::json!({ "mcpServers": { "playwright": { "command": "npx", "args": ["-y", PLAYWRIGHT_MCP, "--config", playwright] } } })
                .to_string(),
        )
        .await?;
        let schema = schema::<T>()?;
        let result = self.run(Provider::Claude, prompt, &schema, &Tools::Browser { mcp: &mcp }, None).await;
        tokio::fs::remove_file(&playwright).await.ok();
        tokio::fs::remove_file(&mcp).await.ok();
        match result {
            Ok(value) => Ok(serde_json::from_value(value)?),
            Err(Failure::Unavailable(why)) => bail!("claude is unavailable: {why}"),
            Err(Failure::Failed(err)) => Err(err),
        }
    }

    async fn run(
        &self,
        provider: Provider,
        prompt: &str,
        schema: &Value,
        tools: &Tools<'_>,
        model: Option<&str>,
    ) -> Result<Value, Failure> {
        let schema_text = schema.to_string();
        let answer_file = tempfile::NamedTempFile::new_in(&self.workdir).map_err(|e| Failure::Failed(e.into()))?;
        let schema_file = tempfile::NamedTempFile::new_in(&self.workdir).map_err(|e| Failure::Failed(e.into()))?;
        tokio::fs::write(schema_file.path(), &schema_text).await.map_err(|e| Failure::Failed(e.into()))?;
        let plain_prompt = format!(
            "{prompt}\n\nReply with one JSON object that matches this JSON Schema, and nothing else:\n{schema_text}"
        );

        let mut command = Command::new(provider.name());
        let model = model.map(|model| ["--model", model]);
        let stdin = match provider {
            Provider::Claude => {
                command
                    .args(["-p", "--output-format", "json", "--no-session-persistence"])
                    .args(model.iter().flatten());
                command.args(["--json-schema", &schema_text]);
                match tools {
                    Tools::Off => command.args(["--tools", ""]),
                    Tools::Web => command.args(["--tools", WEB_TOOLS, "--allowedTools", WEB_TOOLS]),
                    Tools::Browser { mcp } => command
                        .args(["--tools", "", "--strict-mcp-config", "--mcp-config"])
                        .arg(mcp)
                        .args(["--allowedTools", "mcp__playwright"]),
                };
                prompt.to_string()
            }
            Provider::Codex => {
                if let Tools::Web = tools {
                    command.arg("--search");
                }
                command.args([
                    "exec",
                    "--skip-git-repo-check",
                    "--ephemeral",
                    "--sandbox",
                    "read-only",
                    "--output-schema",
                ]);
                command.arg(schema_file.path()).arg("-o").arg(answer_file.path()).args(model.iter().flatten()).arg("-");
                prompt.to_string()
            }
            Provider::Opencode => {
                command.arg("run").args(model.iter().flatten()).arg(&plain_prompt);
                String::new()
            }
            Provider::Gemini => {
                command.args(model.iter().flatten()).arg("-p");
                plain_prompt
            }
        };
        command
            .current_dir(&self.workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|e| Failure::Unavailable(format!("cannot start: {e}")))?;
        let mut pipe = child.stdin.take().expect("stdin is piped");
        pipe.write_all(stdin.as_bytes()).await.map_err(|e| Failure::Failed(e.into()))?;
        drop(pipe);

        let limit = if let Tools::Off = tools { TIMEOUT } else { BROWSE_TIMEOUT };
        let output = tokio::time::timeout(limit, child.wait_with_output())
            .await
            .map_err(|_| Failure::Failed(anyhow!("no answer within {} seconds", limit.as_secs())))?
            .map_err(|e| Failure::Failed(e.into()))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(failure(if stderr.trim().is_empty() { &stdout } else { &stderr }));
        }
        match provider {
            Provider::Claude => {
                let reply: Value = serde_json::from_str(&stdout).map_err(|e| Failure::Failed(e.into()))?;
                if reply["is_error"] == true {
                    return Err(failure(reply["result"].as_str().unwrap_or_default()));
                }
                reply.get("structured_output").cloned().ok_or_else(|| Failure::Failed(anyhow!("no structured output")))
            }
            Provider::Codex => {
                let text =
                    tokio::fs::read_to_string(answer_file.path()).await.map_err(|e| Failure::Failed(e.into()))?;
                json_in(&text).map_err(Failure::Failed)
            }
            Provider::Opencode | Provider::Gemini => json_in(&stdout).map_err(Failure::Failed),
        }
    }
}

/// No `$schema` for claude. Strict for codex: closed objects, all required, `anyOf`.
fn schema<T: JsonSchema>() -> Result<Value> {
    let mut schema = serde_json::to_value(schemars::schema_for!(T))?;
    if let Some(object) = schema.as_object_mut() {
        object.remove("$schema");
    }
    strict(&mut schema);
    Ok(schema)
}

fn strict(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if let Some(choices) = map.remove("oneOf") {
                map.insert("anyOf".into(), choices);
            }
            if let Some(Value::Object(properties)) = map.get("properties") {
                let required = properties.keys().cloned().map(Value::from).collect();
                map.insert("required".into(), Value::Array(required));
                map.insert("additionalProperties".into(), Value::Bool(false));
            }
            map.values_mut().for_each(strict);
        }
        Value::Array(items) => items.iter_mut().for_each(strict),
        _ => {}
    }
}

fn failure(message: &str) -> Failure {
    // codex prints a banner and the prompt first. The cause comes last.
    let message = message.trim();
    let message: String = match message.rfind("ERROR:") {
        Some(at) => message[at..].chars().take(300).collect(),
        None => message.chars().rev().take(300).collect::<Vec<_>>().into_iter().rev().collect(),
    };
    let lower = message.to_lowercase();
    let signs =
        ["usage limit", "rate limit", "quota", "credit", "not logged in", "login", "unauthorized", "subscription"];
    if signs.iter().any(|sign| lower.contains(sign)) {
        Failure::Unavailable(message)
    } else {
        Failure::Failed(anyhow!(message))
    }
}

fn json_in(text: &str) -> Result<Value> {
    let start = text.find('{').context("no JSON object in the answer")?;
    let end = text.rfind('}').context("no JSON object in the answer")?;
    Ok(serde_json::from_str(&text[start..=end])?)
}
