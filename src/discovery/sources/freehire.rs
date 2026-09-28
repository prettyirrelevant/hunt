use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::discovery::model::{Reach, Want};
use crate::jobs::{Posting, Requirement, WorkMode};

const API: &str = "https://freehire.me/api/v1/agent/jobs/search";
const PAGE: usize = 50;
const MAX_PAGES: usize = 4;

pub async fn search(http: &Client, want: &Want) -> Result<Vec<Posting>> {
    let mut postings = vec![];
    for term in &want.terms {
        for scope in scopes(&want.reach) {
            for page in 0..MAX_PAGES {
                let found = fetch(http, want, term, &scope, page).await?;
                let last = found.len() < PAGE;
                postings.extend(found);
                if last {
                    break;
                }
            }
        }
    }
    Ok(postings)
}

fn scopes(reach: &Reach) -> Vec<Vec<(&'static str, String)>> {
    let mut scopes = vec![vec![
        ("work_mode", "remote".into()),
        ("regions", "global".into()),
        ("regions", reach.region().into()),
        ("countries", reach.country.clone()),
    ]];
    if reach.relocate {
        scopes.push(vec![("visa_sponsorship", "true".into())]);
        scopes.push(vec![("relocation", "supported".into())]);
    }
    scopes
}

async fn fetch(http: &Client, want: &Want, term: &str, scope: &[(&str, String)], page: usize) -> Result<Vec<Posting>> {
    let mut query: Vec<(&str, String)> = vec![
        ("q", term.into()),
        ("limit", PAGE.to_string()),
        ("offset", (page * PAGE).to_string()),
        ("posted_within_days", want.days.to_string()),
        ("description_format", "markdown".into()),
        ("is_tech", "tech".into()),
        ("sort", "posted_at".into()),
    ];
    query.extend(scope.iter().cloned());
    query.extend(want.seniority.iter().map(|s| ("seniority", s.clone())));

    let body = http.get(API).query(&query).send().await?.error_for_status()?.bytes().await?;
    parse(&body)
}

pub fn parse(body: &[u8]) -> Result<Vec<Posting>> {
    let response: Response = serde_json::from_slice(body).context("freehire returned an unexpected response")?;
    Ok(response.data.into_iter().map(Job::into_posting).collect())
}

#[derive(Deserialize)]
struct Response {
    data: Vec<Job>,
}

#[derive(Deserialize)]
struct Job {
    public_slug: String,
    source: String,
    url: String,
    title: String,
    company: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    countries: Vec<String>,
    #[serde(default)]
    regions: Vec<String>,
    #[serde(default)]
    work_mode: String,
    #[serde(default)]
    skills: Vec<String>,
    posted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    enrichment: Enrichment,
}

#[derive(Deserialize, Default)]
struct Enrichment {
    visa_sponsorship: Option<bool>,
    relocation: Option<String>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
    salary_currency: Option<String>,
    salary_period: Option<String>,
    seniority: Option<String>,
    #[serde(default)]
    requirements: Vec<FreehireRequirement>,
}

#[derive(Deserialize)]
struct FreehireRequirement {
    text: String,
    priority: String,
}

impl Job {
    fn into_posting(self) -> Posting {
        let e = self.enrichment;
        let url = self.url.split("?utm_source").next().unwrap_or(&self.url);
        let mut p = Posting::new("freehire", &self.public_slug, &self.company, &self.title, url);
        p.source = self.source;
        p.location = self.location;
        p.work_mode = WorkMode::parse(&self.work_mode);
        p.regions = self.regions;
        p.countries = self.countries;
        p.visa = e.visa_sponsorship;
        p.relocation = e.relocation.map(|r| r != "not_supported");
        // Hourly and monthly figures would read as tiny yearly salaries.
        if e.salary_period.as_deref().is_none_or(|p| p == "year") {
            (p.salary_min, p.salary_max) = (e.salary_min, e.salary_max);
        }
        p.currency = e.salary_currency;
        p.seniority = e.seniority;
        p.skills = self.skills;
        p.requirements = e
            .requirements
            .into_iter()
            .map(|r| Requirement { text: r.text, required: r.priority == "required" })
            .collect();
        p.description = self.description;
        p.posted_at = self.posted_at;
        p
    }
}
