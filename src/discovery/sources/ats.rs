use std::fmt;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::common::text::markdown;
use crate::discovery::filter::places;
use crate::jobs::{Posting, WorkMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ats {
    Greenhouse,
    Lever,
    Ashby,
    Workable,
    SmartRecruiters,
    Recruitee,
    Rippling,
    Breezy,
    Workday,
}

/// For Workday the slug is `host/site`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub ats: Ats,
    pub slug: String,
    pub company: String,
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} on {:?}", self.company, self.ats)
    }
}

impl Board {
    pub fn from_url(link: &str) -> Option<Board> {
        let url = url::Url::parse(link.trim()).ok()?;
        let host = url.host_str()?.to_string();
        let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
        let first = segments.first().map(std::string::ToString::to_string);
        let sub = host.split('.').next()?.to_string();

        let (ats, slug) = match host.as_str() {
            "boards-api.greenhouse.io" => (Ats::Greenhouse, segments.get(2)?.to_string()),
            h if h.ends_with("greenhouse.io") => (Ats::Greenhouse, first?),
            "api.lever.co" => (Ats::Lever, segments.get(2)?.to_string()),
            "jobs.lever.co" => (Ats::Lever, first?),
            "jobs.ashbyhq.com" => (Ats::Ashby, first?),
            "apply.workable.com" => (Ats::Workable, first?),
            h if h.ends_with("smartrecruiters.com") => (Ats::SmartRecruiters, first?),
            "ats.rippling.com" => (Ats::Rippling, first?),
            h if h.ends_with(".recruitee.com") => (Ats::Recruitee, sub),
            h if h.ends_with(".breezy.hr") => (Ats::Breezy, sub),
            h if h.ends_with(".myworkdayjobs.com") => {
                let site = segments.iter().find(|s| !s.contains('-') || s.len() > 5)?;
                (Ats::Workday, format!("{host}/{site}"))
            }
            _ => return None,
        };
        let company = titlecase(slug.split('/').next().unwrap_or(&slug).split('.').next().unwrap_or(&slug));
        Some(Board { ats, slug, company })
    }

    pub async fn fetch(&self, http: &Client) -> Result<Vec<Posting>> {
        let body = match self.ats {
            Ats::Workday => {
                let (host, site) = self.slug.split_once('/').context("workday slug needs host/site")?;
                let tenant = host.split('.').next().unwrap_or(host);
                let url = format!("https://{host}/wday/cxs/{tenant}/{site}/jobs");
                let query = json!({ "appliedFacets": {}, "limit": 20, "offset": 0, "searchText": "" });
                http.post(url).json(&query).send().await?.error_for_status()?.bytes().await?
            }
            _ => http.get(self.api()).send().await?.error_for_status()?.bytes().await?,
        };
        self.parse(&body)
    }

    fn api(&self) -> String {
        let slug = &self.slug;
        match self.ats {
            Ats::Greenhouse => {
                format!("https://boards-api.greenhouse.io/v1/boards/{slug}/jobs?content=true")
            }
            Ats::Lever => format!("https://api.lever.co/v0/postings/{slug}?mode=json"),
            Ats::Ashby => format!("https://api.ashbyhq.com/posting-api/job-board/{slug}?includeCompensation=true"),
            Ats::Workable => {
                format!("https://apply.workable.com/api/v1/widget/accounts/{slug}?details=true")
            }
            Ats::SmartRecruiters => {
                format!("https://api.smartrecruiters.com/v1/companies/{slug}/postings?limit=100")
            }
            Ats::Recruitee => format!("https://{slug}.recruitee.com/api/offers/"),
            Ats::Rippling => {
                format!("https://ats.rippling.com/api/v2/board/{slug}/jobs?page=0&pageSize=100")
            }
            Ats::Breezy => format!("https://{slug}.breezy.hr/json"),
            Ats::Workday => String::new(),
        }
    }

    pub fn parse(&self, body: &[u8]) -> Result<Vec<Posting>> {
        let postings = match self.ats {
            Ats::Greenhouse => {
                serde_json::from_slice::<greenhouse::Board>(body)?.jobs.into_iter().map(|j| j.posting(self)).collect()
            }
            Ats::Lever => {
                serde_json::from_slice::<Vec<lever::Job>>(body)?.into_iter().map(|j| j.posting(self)).collect()
            }
            Ats::Ashby => serde_json::from_slice::<ashby::Board>(body)?
                .jobs
                .into_iter()
                .filter(|j| j.is_listed)
                .map(|j| j.posting(self))
                .collect(),
            Ats::Workable => {
                serde_json::from_slice::<workable::Board>(body)?.jobs.into_iter().map(|j| j.posting(self)).collect()
            }
            Ats::SmartRecruiters => serde_json::from_slice::<smartrecruiters::Board>(body)?
                .content
                .into_iter()
                .map(|j| j.posting(self))
                .collect(),
            Ats::Recruitee => {
                serde_json::from_slice::<recruitee::Board>(body)?.offers.into_iter().map(|j| j.posting(self)).collect()
            }
            Ats::Rippling => {
                serde_json::from_slice::<rippling::Board>(body)?.items.into_iter().map(|j| j.posting(self)).collect()
            }
            Ats::Breezy => {
                serde_json::from_slice::<Vec<breezy::Job>>(body)?.into_iter().map(|j| j.posting(self)).collect()
            }
            Ats::Workday => serde_json::from_slice::<workday::Board>(body)?
                .job_postings
                .into_iter()
                .map(|j| j.posting(self))
                .collect(),
        };
        Ok(postings)
    }

    fn posting(&self, id: impl fmt::Display, title: &str, url: &str) -> Posting {
        let source = format!("{:?}", self.ats).to_lowercase();
        let mut p = Posting::new(&source, format!("{}:{id}", self.slug), &self.company, title, url);
        p.work_mode = WorkMode::Unknown;
        p
    }
}

fn locate(p: &mut Posting, location: String, remote: Option<bool>) {
    let lower = location.to_lowercase();
    p.work_mode = match remote {
        Some(true) => WorkMode::Remote,
        _ if lower.contains("remote") => WorkMode::Remote,
        _ if lower.contains("hybrid") => WorkMode::Hybrid,
        _ if location.is_empty() && remote.is_none() => WorkMode::Unknown,
        _ => WorkMode::Onsite,
    };
    (p.regions, p.countries) = places(&location);
    p.location = location;
}

fn titlecase(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| w[..1].to_uppercase() + &w[1..])
        .collect::<Vec<_>>()
        .join(" ")
}

mod greenhouse {
    use super::{DateTime, Deserialize, Posting, Utc, locate, markdown};

    #[derive(Deserialize)]
    pub struct Board {
        pub jobs: Vec<Job>,
    }

    #[derive(Deserialize)]
    pub struct Job {
        id: u64,
        title: String,
        absolute_url: String,
        location: Option<Named>,
        #[serde(default)]
        content: String,
        first_published: Option<DateTime<Utc>>,
        company_name: Option<String>,
    }

    #[derive(Deserialize)]
    struct Named {
        name: String,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let mut p = board.posting(self.id, &self.title, &self.absolute_url);
            p.company = self.company_name.unwrap_or(p.company);
            p.description = markdown(&self.content);
            p.posted_at = self.first_published;
            locate(&mut p, self.location.map(|l| l.name).unwrap_or_default(), None);
            p
        }
    }
}

mod lever {
    use super::{Deserialize, Posting, TimeZone, Utc, locate, markdown};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Job {
        id: String,
        text: String,
        hosted_url: String,
        apply_url: Option<String>,
        #[serde(default)]
        categories: Categories,
        workplace_type: Option<String>,
        #[serde(default)]
        description_plain: String,
        #[serde(default)]
        lists: Vec<List>,
        #[serde(default)]
        additional_plain: String,
        created_at: Option<i64>,
        salary_range: Option<Salary>,
    }

    #[derive(Deserialize, Default)]
    struct Categories {
        location: Option<String>,
    }

    #[derive(Deserialize)]
    struct List {
        text: String,
        content: String,
    }

    #[derive(Deserialize)]
    struct Salary {
        min: Option<i64>,
        max: Option<i64>,
        currency: Option<String>,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let mut p = board.posting(&self.id, &self.text, &self.hosted_url);
            p.apply_url = self.apply_url.unwrap_or(p.apply_url);
            let lists = self.lists.iter().map(|l| format!("## {}\n\n{}", l.text, markdown(&l.content)));
            p.description = std::iter::once(self.description_plain)
                .chain(lists)
                .chain(std::iter::once(self.additional_plain))
                .collect::<Vec<_>>()
                .join("\n\n")
                .trim()
                .to_string();
            p.posted_at = self.created_at.and_then(|ms| Utc.timestamp_millis_opt(ms).single());
            if let Some(s) = self.salary_range {
                (p.salary_min, p.salary_max, p.currency) = (s.min, s.max, s.currency);
            }
            let remote = self.workplace_type.map(|w| w == "remote");
            locate(&mut p, self.categories.location.unwrap_or_default(), remote);
            p
        }
    }
}

mod ashby {
    use super::{DateTime, Deserialize, Posting, Utc, locate, markdown};

    #[derive(Deserialize)]
    pub struct Board {
        pub jobs: Vec<Job>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Job {
        id: String,
        title: String,
        #[serde(default)]
        location: String,
        is_remote: Option<bool>,
        #[serde(default)]
        pub is_listed: bool,
        #[serde(default)]
        description_html: String,
        #[serde(rename = "jobUrl")]
        url: String,
        apply_url: Option<String>,
        published_at: Option<DateTime<Utc>>,
        compensation: Option<Compensation>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Compensation {
        compensation_tier_summary: Option<String>,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let mut p = board.posting(&self.id, &self.title, &self.url);
            p.apply_url = self.apply_url.unwrap_or(p.apply_url);
            p.description = markdown(&self.description_html);
            if let Some(pay) = self.compensation.and_then(|c| c.compensation_tier_summary) {
                p.description = format!("**Compensation:** {pay}\n\n{}", p.description);
            }
            p.posted_at = self.published_at;
            locate(&mut p, self.location, self.is_remote);
            p
        }
    }
}

mod workable {
    use super::{Deserialize, Posting, locate, markdown};

    #[derive(Deserialize)]
    pub struct Board {
        pub jobs: Vec<Job>,
    }

    #[derive(Deserialize)]
    pub struct Job {
        shortcode: String,
        title: String,
        url: String,
        application_url: Option<String>,
        #[serde(default)]
        country: String,
        #[serde(default)]
        city: String,
        #[serde(default)]
        telecommuting: bool,
        #[serde(default)]
        description: String,
        published_on: Option<chrono::NaiveDate>,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let mut p = board.posting(&self.shortcode, &self.title, &self.url);
            p.apply_url = self.application_url.unwrap_or(p.apply_url);
            p.description = markdown(&self.description);
            p.posted_at = self.published_on.and_then(|d| d.and_hms_opt(0, 0, 0)).map(|d| d.and_utc());
            let location = [self.city, self.country].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>();
            locate(&mut p, location.join(", "), self.telecommuting.then_some(true));
            p
        }
    }
}

mod smartrecruiters {
    use super::{DateTime, Deserialize, Posting, Utc, locate};

    #[derive(Deserialize)]
    pub struct Board {
        pub content: Vec<Job>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Job {
        id: String,
        name: String,
        released_date: Option<DateTime<Utc>>,
        location: Option<Location>,
        company: Option<Company>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Location {
        full_location: Option<String>,
        remote: Option<bool>,
    }

    #[derive(Deserialize)]
    struct Company {
        identifier: String,
        name: Option<String>,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let (identifier, name) = match self.company {
                Some(c) => (c.identifier, c.name),
                None => (board.slug.clone(), None),
            };
            let url = format!("https://jobs.smartrecruiters.com/{identifier}/{}", self.id);
            let mut p = board.posting(&self.id, &self.name, &url);
            p.company = name.unwrap_or(p.company);
            p.posted_at = self.released_date;
            let (location, remote) =
                self.location.map(|l| (l.full_location.unwrap_or_default(), l.remote)).unwrap_or_default();
            locate(&mut p, location, remote);
            p
        }
    }
}

mod recruitee {
    use super::{Deserialize, NaiveDateTime, Posting, WorkMode, locate, markdown};

    #[derive(Deserialize)]
    pub struct Board {
        pub offers: Vec<Offer>,
    }

    #[derive(Deserialize)]
    pub struct Offer {
        id: u64,
        title: String,
        company_name: Option<String>,
        careers_url: String,
        careers_apply_url: Option<String>,
        #[serde(default)]
        location: String,
        #[serde(default)]
        remote: bool,
        #[serde(default)]
        hybrid: bool,
        #[serde(default)]
        description: String,
        #[serde(default)]
        requirements: String,
        published_at: Option<String>,
        salary: Option<Salary>,
    }

    #[derive(Deserialize)]
    struct Salary {
        min: Option<String>,
        max: Option<String>,
        period: Option<String>,
        currency: Option<String>,
    }

    impl Offer {
        pub fn posting(self, board: &super::Board) -> Posting {
            let mut p = board.posting(self.id, &self.title, &self.careers_url);
            p.company = self.company_name.unwrap_or(p.company);
            p.apply_url = self.careers_apply_url.unwrap_or(p.apply_url);
            p.description =
                format!("{}\n\n{}", markdown(&self.description), markdown(&self.requirements)).trim().into();
            p.posted_at = self
                .published_at
                .and_then(|d| NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M:%S UTC").ok())
                .map(|d| d.and_utc());
            if let Some(s) = self.salary.filter(|s| s.period.as_deref() == Some("year")) {
                p.salary_min = s.min.and_then(|n| n.parse().ok());
                p.salary_max = s.max.and_then(|n| n.parse().ok());
                p.currency = s.currency;
            }
            locate(&mut p, self.location, Some(self.remote));
            if self.hybrid {
                p.work_mode = WorkMode::Hybrid;
            }
            p
        }
    }
}

mod rippling {
    use super::{Deserialize, Posting, locate};

    #[derive(Deserialize)]
    pub struct Board {
        pub items: Vec<Job>,
    }

    #[derive(Deserialize)]
    pub struct Job {
        id: String,
        name: String,
        url: String,
        #[serde(default)]
        locations: Vec<Location>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Location {
        name: String,
        workplace_type: Option<String>,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let mut p = board.posting(&self.id, &self.name, &self.url);
            let remote = self.locations.iter().any(|l| l.workplace_type.as_deref() == Some("REMOTE"));
            let names = self.locations.into_iter().map(|l| l.name).collect::<Vec<_>>().join("; ");
            locate(&mut p, names, Some(remote));
            p
        }
    }
}

mod breezy {
    use super::{DateTime, Deserialize, Posting, Utc, locate};

    #[derive(Deserialize)]
    pub struct Job {
        id: String,
        name: String,
        url: String,
        published_date: Option<DateTime<Utc>>,
        location: Option<Location>,
        company: Option<Company>,
    }

    #[derive(Deserialize)]
    struct Location {
        name: Option<String>,
        is_remote: Option<bool>,
    }

    #[derive(Deserialize)]
    struct Company {
        name: String,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let mut p = board.posting(&self.id, &self.name, &self.url);
            p.company = self.company.map(|c| c.name).unwrap_or(p.company);
            p.posted_at = self.published_date;
            let (name, remote) = self.location.map(|l| (l.name.unwrap_or_default(), l.is_remote)).unwrap_or_default();
            locate(&mut p, name, remote);
            p
        }
    }
}

mod workday {
    use super::{Deserialize, Posting, locate};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Board {
        pub job_postings: Vec<Job>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Job {
        title: String,
        external_path: String,
        #[serde(default)]
        locations_text: String,
    }

    impl Job {
        pub fn posting(self, board: &super::Board) -> Posting {
            let (host, site) = board.slug.split_once('/').unwrap_or((&board.slug, ""));
            let url = format!("https://{host}/{site}{}", self.external_path);
            let mut p = board.posting(&self.external_path, &self.title, &url);
            // "Poland-Remote" style paths carry more than "5 Locations" does.
            let hint = self.external_path.split('/').nth(2).unwrap_or_default().replace('-', " ");
            locate(&mut p, format!("{} {hint}", self.locations_text).trim().to_string(), None);
            p
        }
    }
}
