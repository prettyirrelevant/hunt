use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use tokio::process::Command;
use walkdir::WalkDir;

const SKIP: [&str; 6] = ["node_modules", "target", "vendor", "dist", "build", ".venv"];
const MANIFESTS: [&str; 5] = ["Cargo.toml", "package.json", "pyproject.toml", "go.mod", "Gemfile"];

/// Your "never read" patterns, read like a .gitignore in your home folder.
/// `~/` anchors a pattern there.
pub struct NeverRead(Gitignore);

impl NeverRead {
    pub fn new(patterns: &[String]) -> Result<NeverRead> {
        let mut builder = GitignoreBuilder::new(home());
        for pattern in patterns {
            let pattern = pattern.strip_prefix('~').unwrap_or(pattern);
            builder.add_line(None, pattern)?;
        }
        Ok(NeverRead(builder.build()?))
    }

    pub fn covers(&self, path: &Path) -> bool {
        path.ancestors().any(|p| self.0.matched(p, p != path || path.is_dir()).is_ignore())
    }
}

fn home() -> PathBuf {
    directories::BaseDirs::new().expect("a home directory").home_dir().to_path_buf()
}

pub fn find_repos(roots: &[PathBuf], never: &NeverRead) -> Vec<PathBuf> {
    let mut repos = vec![];
    for root in roots {
        let mut walk = WalkDir::new(root).max_depth(4).into_iter();
        while let Some(entry) = walk.next() {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            if SKIP.contains(&name.as_ref())
                || (name.starts_with('.') && entry.depth() > 0)
                || never.covers(entry.path())
            {
                walk.skip_current_dir();
            } else if entry.path().join(".git").exists() {
                repos.push(entry.path().to_path_buf());
                walk.skip_current_dir();
            }
        }
    }
    repos
}

/// `None` when you never committed to the repo.
pub async fn read_repo(repo: &Path, never: &NeverRead) -> Result<Option<String>> {
    let email = git(repo, &["config", "user.email"]).await.unwrap_or_default();
    if email.is_empty() {
        return Ok(None);
    }
    let log = git(repo, &["log", "--no-merges", &format!("--author={email}"), "--format=%h %as %s"]).await?;
    let commits: Vec<&str> = log.lines().collect();
    let (Some(newest), Some(oldest)) = (commits.first(), commits.last()) else {
        return Ok(None);
    };

    let name = repo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut body = format!(
        "Repository: {name}\nYour commits: {} between {} and {}\nLanguages: {}\n",
        commits.len(),
        oldest.split(' ').nth(1).unwrap_or_default(),
        newest.split(' ').nth(1).unwrap_or_default(),
        languages(repo, never).await?,
    );
    for manifest in MANIFESTS.iter().filter(|m| !never.covers(&repo.join(m))) {
        if let Ok(text) = tokio::fs::read_to_string(repo.join(manifest)).await {
            let _ = write!(body, "\n{manifest}:\n{}\n", text.chars().take(1500).collect::<String>());
        }
    }
    for readme in ["README.md", "readme.md", "README"].iter().filter(|r| !never.covers(&repo.join(r))) {
        if let Ok(text) = tokio::fs::read_to_string(repo.join(readme)).await {
            let _ = write!(body, "\nREADME:\n{}\n", text.chars().take(3000).collect::<String>());
            break;
        }
    }
    body.push_str("\nYour commit messages, newest first:\n");
    for commit in commits.iter().take(120) {
        let _ = writeln!(body, "- {}", commit.splitn(3, ' ').nth(2).unwrap_or_default());
    }
    Ok(Some(body))
}

async fn languages(repo: &Path, never: &NeverRead) -> Result<String> {
    let files = git(repo, &["ls-files"]).await?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let kept = files.lines().filter(|f| !never.covers(&repo.join(f)));
    for ext in kept.filter_map(|f| Path::new(f).extension()) {
        *counts.entry(ext.to_string_lossy().to_lowercase()).or_default() += 1;
    }
    let total: usize = counts.values().sum::<usize>().max(1);
    let mut ranked: Vec<_> = counts.into_iter().collect();
    ranked.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    Ok(ranked.iter().take(6).map(|(ext, n)| format!("{ext} {}%", n * 100 / total)).collect::<Vec<_>>().join(", "))
}

async fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").arg("-C").arg(repo).args(args).output().await.context("git is not installed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn read_cv(name: &str, bytes: &[u8]) -> Result<String> {
    let text = if name.to_lowercase().ends_with(".pdf") {
        pdf_oxide::PdfDocument::from_bytes(bytes.to_vec())
            .and_then(|pdf| pdf.extract_all_text())
            .context("could not read that PDF")?
    } else {
        String::from_utf8(bytes.to_vec()).context("the CV is not a PDF or a text file")?
    };
    Ok(text.trim().to_string())
}
