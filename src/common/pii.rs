use std::{fmt::Write as _, path::Path, time::Duration};

use anyhow::{Context, Result, bail};
use gliner::model::{GLiNER, input::text::TextInput, params::Parameters, pipeline::span::SpanMode};
use orp::params::RuntimeParameters;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::text::scrub;

/// Pinned to one commit and checked by hash, so a changed upload never reaches you.
const SOURCE: &str =
    "https://huggingface.co/gravitee-io/gliner-pii-detection/resolve/e6282f2fa32fa6098f64043c6afe0b53e9ae8db7";
/// Remote name, local name, SHA-256.
const FILES: [(&str, &str, &str); 2] = [
    ("tokenizer.json", "tokenizer.json", "953deb290bdcbd985bb91223801eeb4bea79021f0d2e24ebb764dcd2b0816cf9"),
    ("model.quant.onnx", "model.onnx", "e73d9aaf0f667aa9b7d52fedd0259931d3bee025876ebe364a25d5730cb7a1f7"),
];

/// Company and product names stay: they belong on a CV.
const LABELS: [(&str, &str); 8] = [
    ("name", "[name]"),
    ("email", "[email]"),
    ("phone_number", "[phone]"),
    ("street_address", "[address]"),
    ("date_of_birth", "[date of birth]"),
    ("ipv4", "[ip]"),
    ("ssn", "[id number]"),
    ("passport_number", "[id number]"),
];
const THRESHOLD: f32 = 0.5;
const CHUNK: usize = 1200;

pub struct Redactor(GLiNER<SpanMode>);

impl Redactor {
    /// Downloads the model into `dir` the first time, then loads it.
    pub async fn fetch(http: &reqwest::Client, dir: &Path) -> Result<Redactor> {
        tokio::fs::create_dir_all(dir).await?;
        for (remote, local, sha256) in FILES {
            let path = dir.join(local);
            if !tokio::fs::try_exists(&path).await? {
                download(http, &format!("{SOURCE}/{remote}"), &path, sha256).await?;
            }
        }
        let dir = dir.to_path_buf();
        tokio::task::spawn_blocking(move || {
            GLiNER::<SpanMode>::new(
                Parameters::default(),
                RuntimeParameters::default(),
                dir.join("tokenizer.json"),
                dir.join("model.onnx"),
            )
            .map(Redactor)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("could not load the PII model")
        })
        .await?
    }

    /// CPU-bound: call it from `spawn_blocking`.
    pub fn redact(&self, text: &str) -> Result<String> {
        let chunks = chunks(&scrub(text));
        let labels: Vec<&str> = LABELS.iter().map(|(label, _)| *label).collect();
        let input = TextInput::from_str(&chunks.iter().map(String::as_str).collect::<Vec<_>>(), &labels)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let output = self.0.inference(input).map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut redacted = String::with_capacity(text.len());
        for (chunk, mut spans) in chunks.into_iter().zip(output.spans) {
            spans.retain(|s| s.probability() >= THRESHOLD);
            spans.sort_by_key(|s| std::cmp::Reverse(s.offsets().0));
            let mut chunk = chunk;
            for span in spans {
                let (start, end) = span.offsets();
                let mask = LABELS.iter().find(|(label, _)| *label == span.class()).map_or("[private]", |(_, m)| m);
                if chunk.is_char_boundary(start) && chunk.is_char_boundary(end) && start < end {
                    chunk.replace_range(start..end, mask);
                }
            }
            redacted.push_str(&chunk);
        }
        Ok(redacted)
    }
}

/// Writes to a temporary file and renames it only when the hash matches, so a
/// cut connection never leaves a broken model behind.
async fn download(http: &reqwest::Client, url: &str, path: &Path, sha256: &str) -> Result<()> {
    let partial = path.with_extension("partial");
    let mut response =
        http.get(url).timeout(Duration::from_secs(900)).send().await?.error_for_status().context("download failed")?;
    let mut file = tokio::fs::File::create(&partial).await?;
    let mut hasher = Sha256::new();
    while let Some(chunk) = response.chunk().await? {
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    let got = hasher.finalize().iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    if got != sha256 {
        tokio::fs::remove_file(&partial).await.ok();
        bail!("{url} did not match its pinned hash");
    }
    tokio::fs::rename(&partial, path).await?;
    Ok(())
}

fn chunks(text: &str) -> Vec<String> {
    let mut chunks = vec![String::new()];
    for line in text.split_inclusive('\n') {
        let current = chunks.last_mut().expect("never empty");
        if !current.is_empty() && current.len() + line.len() > CHUNK {
            chunks.push(line.to_string());
        } else {
            current.push_str(line);
        }
    }
    chunks
}
