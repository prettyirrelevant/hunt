use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::common::text::markdown;
use crate::discovery::filter::places;
use crate::discovery::model::Want;
use crate::jobs::{Posting, WorkMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feed {
    Himalayas,
    Jobicy,
    Remotive,
    RemoteOk,
    WeWorkRemotely,
    WorkingNomads,
    Arbeitnow,
    LandingJobs,
    FourDayWeek,
    HackerNews,
    LinkedIn,
}

impl Feed {
    pub const ALL: [Feed; 11] = [
        Feed::Himalayas,
        Feed::Jobicy,
        Feed::Remotive,
        Feed::RemoteOk,
        Feed::WeWorkRemotely,
        Feed::WorkingNomads,
        Feed::Arbeitnow,
        Feed::LandingJobs,
        Feed::FourDayWeek,
        Feed::HackerNews,
        Feed::LinkedIn,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Feed::Himalayas => "Himalayas",
            Feed::Jobicy => "Jobicy",
            Feed::Remotive => "Remotive",
            Feed::RemoteOk => "Remote OK",
            Feed::WeWorkRemotely => "We Work Remotely",
            Feed::WorkingNomads => "Working Nomads",
            Feed::Arbeitnow => "Arbeitnow",
            Feed::LandingJobs => "Landing.jobs",
            Feed::FourDayWeek => "4 Day Week",
            Feed::HackerNews => "Hacker News",
            Feed::LinkedIn => "LinkedIn",
        }
    }

    pub async fn fetch(self, http: &Client, want: &Want) -> Result<Vec<Posting>> {
        let mut postings = vec![];
        for url in self.urls(want) {
            let body = http.get(&url).send().await?.error_for_status()?.bytes().await?;
            if self == Feed::HackerNews && url.contains("search_by_date") {
                let story = latest_hiring_thread(&body)?;
                let item = format!("https://hn.algolia.com/api/v1/items/{story}");
                let body = http.get(item).send().await?.error_for_status()?.bytes().await?;
                postings.extend(self.parse(&body)?);
                continue;
            }
            postings.extend(self.parse(&body)?);
        }
        postings.retain(|p| want.matches(&p.title, &p.skills));
        Ok(postings)
    }

    fn urls(self, want: &Want) -> Vec<String> {
        let one = |url: &str| vec![url.to_string()];
        match self {
            Feed::Himalayas => want
                .terms
                .iter()
                .map(|term| {
                    let q = url::form_urlencoded::byte_serialize(term.as_bytes()).collect::<String>();
                    format!("https://himalayas.app/jobs/api/search?q={q}&limit=100")
                })
                .collect(),
            Feed::Jobicy => one("https://jobicy.com/api/v2/remote-jobs?count=100&industry=dev"),
            Feed::Remotive => one("https://remotive.com/api/remote-jobs?category=software-dev"),
            Feed::RemoteOk => one("https://remoteok.com/api"),
            Feed::WeWorkRemotely => ["programming", "devops-sysadmin", "full-stack-programming", "back-end-programming"]
                .iter()
                .map(|c| format!("https://weworkremotely.com/categories/remote-{c}-jobs.rss"))
                .collect(),
            Feed::WorkingNomads => one("https://www.workingnomads.com/api/exposed_jobs/"),
            Feed::Arbeitnow => (1..=3).map(|p| format!("https://www.arbeitnow.com/api/job-board-api?page={p}")).collect(),
            Feed::LandingJobs => (0..3).map(|p| format!("https://landing.jobs/api/v1/jobs?limit=50&offset={}", p * 50)).collect(),
            Feed::FourDayWeek => one("https://4dayweek.io/api/jobs"),
            Feed::HackerNews => one(
                "https://hn.algolia.com/api/v1/search_by_date?tags=story,author_whoishiring&query=who%20is%20hiring&hitsPerPage=1",
            ),
            Feed::LinkedIn => want
                .terms
                .iter()
                .map(|term| {
                    let q = url::form_urlencoded::byte_serialize(term.as_bytes()).collect::<String>();
                    format!("https://www.linkedin.com/jobs-guest/jobs/api/seeMoreJobPostings/search?keywords={q}&location=Worldwide&f_WT=2&f_TPR=r604800&start=0")
                })
                .collect(),
        }
    }

    pub fn parse(self, body: &[u8]) -> Result<Vec<Posting>> {
        Ok(match self {
            Feed::Himalayas => lenient::<Himalayas>(serde_json::from_slice::<ListOf>(body)?.jobs),
            Feed::Jobicy => lenient::<Jobicy>(serde_json::from_slice::<ListOf>(body)?.jobs),
            Feed::Remotive => lenient::<Remotive>(serde_json::from_slice::<ListOf>(body)?.jobs),
            Feed::RemoteOk => {
                // The first element is the API's legal notice.
                let items: Vec<serde_json::Value> = serde_json::from_slice(body)?;
                lenient::<RemoteOk>(items.into_iter().skip(1).collect())
            }
            Feed::WeWorkRemotely => rss(body)?,
            Feed::WorkingNomads => lenient::<WorkingNomads>(serde_json::from_slice(body)?),
            Feed::Arbeitnow => lenient::<Arbeitnow>(serde_json::from_slice::<DataOf>(body)?.data),
            Feed::LandingJobs => lenient::<LandingJobs>(serde_json::from_slice(body)?),
            Feed::FourDayWeek => lenient::<FourDayWeek>(serde_json::from_slice::<ListOf>(body)?.jobs),
            Feed::HackerNews => hacker_news(body)?,
            Feed::LinkedIn => linkedin(body),
        })
    }
}

#[derive(Deserialize)]
struct ListOf {
    jobs: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DataOf {
    data: Vec<serde_json::Value>,
}

/// Parses each posting on its own and skips malformed ones.
fn lenient<T: serde::de::DeserializeOwned + Into<Posting>>(items: Vec<serde_json::Value>) -> Vec<Posting> {
    items.into_iter().filter_map(|item| serde_json::from_value::<T>(item).ok()).map(Into::into).collect()
}

fn remote(mut p: Posting, location: &str) -> Posting {
    p.work_mode = WorkMode::Remote;
    (p.regions, p.countries) = places(location);
    p.location = location.to_string();
    p
}

fn yearly(p: &mut Posting, min: Option<i64>, max: Option<i64>, period: Option<&str>, currency: Option<String>) {
    if period.is_none_or(|p| matches!(p, "year" | "yearly" | "annual")) {
        (p.salary_min, p.salary_max) = (min.filter(|n| *n > 0), max.filter(|n| *n > 0));
        p.currency = currency;
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Himalayas {
    guid: String,
    title: String,
    company_name: String,
    application_link: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    location_restrictions: Vec<String>,
    min_salary: Option<i64>,
    max_salary: Option<i64>,
    salary_period: Option<String>,
    currency: Option<String>,
    #[serde(default)]
    seniority: Vec<String>,
    #[serde(default)]
    categories: Vec<String>,
    pub_date: Option<i64>,
}

impl From<Himalayas> for Posting {
    fn from(j: Himalayas) -> Posting {
        let place =
            if j.location_restrictions.is_empty() { "Worldwide".into() } else { j.location_restrictions.join(", ") };
        let mut p = remote(Posting::new("himalayas", &j.guid, &j.company_name, &j.title, &j.guid), &place);
        p.apply_url = j.application_link;
        p.description = markdown(&j.description);
        yearly(&mut p, j.min_salary, j.max_salary, j.salary_period.as_deref(), j.currency);
        p.seniority = j.seniority.into_iter().next();
        p.skills = j.categories.into_iter().map(|c| c.replace('-', " ").to_lowercase()).collect();
        p.posted_at = j.pub_date.and_then(|s| Utc.timestamp_opt(s, 0).single());
        p
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Jobicy {
    id: u64,
    url: String,
    job_title: String,
    company_name: String,
    #[serde(default)]
    job_geo: String,
    #[serde(default)]
    job_level: String,
    #[serde(default)]
    job_description: String,
    pub_date: Option<DateTime<Utc>>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
    salary_currency: Option<String>,
    salary_period: Option<String>,
}

impl From<Jobicy> for Posting {
    fn from(j: Jobicy) -> Posting {
        let place = if j.job_geo == "Anywhere" { "Worldwide" } else { j.job_geo.as_str() };
        let mut p = remote(Posting::new("jobicy", j.id, &j.company_name, &j.job_title, &j.url), place);
        p.description = markdown(&j.job_description);
        p.seniority = Some(j.job_level).filter(|l| l != "Any");
        yearly(&mut p, j.salary_min, j.salary_max, j.salary_period.as_deref(), j.salary_currency);
        p.posted_at = j.pub_date;
        p
    }
}

#[derive(Deserialize)]
struct Remotive {
    id: u64,
    url: String,
    title: String,
    company_name: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    candidate_required_location: String,
    #[serde(default)]
    description: String,
    publication_date: Option<String>,
}

impl From<Remotive> for Posting {
    fn from(j: Remotive) -> Posting {
        let mut p =
            remote(Posting::new("remotive", j.id, &j.company_name, &j.title, &j.url), &j.candidate_required_location);
        p.description = markdown(&j.description);
        p.skills = j.tags.into_iter().map(|t| t.to_lowercase()).collect();
        p.posted_at = j.publication_date.and_then(|d| d.parse::<chrono::NaiveDateTime>().ok()).map(|d| d.and_utc());
        p
    }
}

#[derive(Deserialize)]
struct RemoteOk {
    id: String,
    url: String,
    position: String,
    company: String,
    apply_url: Option<String>,
    #[serde(default)]
    location: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    description: String,
    date: Option<DateTime<Utc>>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
}

impl From<RemoteOk> for Posting {
    fn from(j: RemoteOk) -> Posting {
        let place = if j.location.is_empty() { "Worldwide" } else { j.location.as_str() };
        let mut p = remote(Posting::new("remoteok", &j.id, &j.company, &j.position, &j.url), place);
        p.apply_url = j.apply_url.unwrap_or(p.apply_url);
        p.description = markdown(&j.description);
        p.skills = j.tags;
        yearly(&mut p, j.salary_min, j.salary_max, None, Some("USD".into()));
        p.posted_at = j.date;
        p
    }
}

#[derive(Deserialize)]
struct WorkingNomads {
    url: String,
    title: String,
    company_name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    tags: String,
    pub_date: Option<DateTime<Utc>>,
}

impl From<WorkingNomads> for Posting {
    fn from(j: WorkingNomads) -> Posting {
        // Titles arrive as "Company - Role".
        let title = j.title.split_once(" - ").map_or(j.title.as_str(), |(_, role)| role);
        let mut p = remote(Posting::new("workingnomads", &j.url, &j.company_name, title, &j.url), &j.location);
        p.description = markdown(&j.description);
        p.skills = j.tags.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        p.posted_at = j.pub_date;
        p
    }
}

#[derive(Deserialize)]
struct Arbeitnow {
    slug: String,
    url: String,
    title: String,
    company_name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    remote: bool,
    #[serde(default)]
    location: String,
    #[serde(default)]
    tags: Vec<String>,
    created_at: Option<i64>,
}

impl From<Arbeitnow> for Posting {
    fn from(j: Arbeitnow) -> Posting {
        let mut p = Posting::new("arbeitnow", &j.slug, &j.company_name, &j.title, &j.url);
        p.work_mode = if j.remote { WorkMode::Remote } else { WorkMode::Onsite };
        (p.regions, p.countries) = places(&format!("{}, Germany", j.location));
        p.location = j.location;
        p.description = markdown(&j.description);
        p.skills = j.tags;
        p.posted_at = j.created_at.and_then(|s| Utc.timestamp_opt(s, 0).single());
        p
    }
}

#[derive(Deserialize)]
struct LandingJobs {
    id: u64,
    url: String,
    title: String,
    role_description: Option<String>,
    main_requirements: Option<String>,
    #[serde(default)]
    remote: bool,
    #[serde(default)]
    relocation_paid: bool,
    #[serde(default)]
    locations: Vec<LandingLocation>,
    gross_salary_low: Option<i64>,
    gross_salary_high: Option<i64>,
    currency_code: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    published_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct LandingLocation {
    city: Option<String>,
    country_code: Option<String>,
}

impl From<LandingJobs> for Posting {
    fn from(j: LandingJobs) -> Posting {
        // "https://landing.jobs/at/{company}/{role}"
        let company = j.url.split('/').nth(4).unwrap_or("").replace('-', " ");
        let mut p = Posting::new("landingjobs", j.id, &company, &j.title, &j.url);
        p.work_mode = if j.remote { WorkMode::Remote } else { WorkMode::Onsite };
        p.relocation = Some(j.relocation_paid);
        p.countries = j.locations.iter().filter_map(|l| l.country_code.as_ref().map(|c| c.to_lowercase())).collect();
        p.location = j.locations.iter().filter_map(|l| l.city.clone()).collect::<Vec<_>>().join(", ");
        p.description = format!(
            "{}\n\n## Requirements\n\n{}",
            markdown(j.role_description.as_deref().unwrap_or_default()),
            markdown(j.main_requirements.as_deref().unwrap_or_default())
        );
        yearly(&mut p, j.gross_salary_low, j.gross_salary_high, None, j.currency_code);
        p.skills = j.tags.into_iter().map(|t| t.to_lowercase()).collect();
        p.posted_at = j.published_at;
        p
    }
}

#[derive(Deserialize)]
struct FourDayWeek {
    id: String,
    slug: String,
    title: String,
    company_name: String,
    #[serde(default)]
    work_arrangement: String,
    #[serde(default)]
    locations: Vec<FourDayLocation>,
    salary_lower: Option<i64>,
    salary_upper: Option<i64>,
    salary_currency: Option<String>,
    salary_period: Option<String>,
    level: Option<String>,
    posted: Option<i64>,
}

#[derive(Deserialize)]
struct FourDayLocation {
    country: Option<String>,
}

impl From<FourDayWeek> for Posting {
    fn from(j: FourDayWeek) -> Posting {
        let url = format!("https://4dayweek.io/remote-job/{}", j.slug);
        let mut p = Posting::new("4dayweek", &j.id, &j.company_name, &j.title, &url);
        let place = j.locations.iter().filter_map(|l| l.country.clone()).collect::<Vec<_>>().join(", ");
        p = if j.work_arrangement == "remote" { remote(p, &place) } else { p };
        // Salaries arrive in cents.
        let dollars = |n: Option<i64>| n.map(|c| c / 100);
        yearly(&mut p, dollars(j.salary_lower), dollars(j.salary_upper), j.salary_period.as_deref(), j.salary_currency);
        p.seniority = j.level;
        p.posted_at = j.posted.and_then(|s| Utc.timestamp_opt(s, 0).single());
        p
    }
}

#[derive(Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Deserialize)]
struct Channel {
    #[serde(rename = "item", default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    title: String,
    link: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "pubDate")]
    pub_date: Option<String>,
    region: Option<String>,
}

fn rss(body: &[u8]) -> Result<Vec<Posting>> {
    let feed: Rss = quick_xml::de::from_reader(body).context("unreadable RSS feed")?;
    Ok(feed
        .channel
        .items
        .into_iter()
        .map(|item| {
            let (company, title) = item.title.split_once(": ").unwrap_or(("", &item.title));
            let place = item.region.as_deref().unwrap_or("Anywhere in the World");
            let mut p = remote(Posting::new("weworkremotely", &item.link, company, title, &item.link), place);
            p.description = markdown(&item.description);
            p.posted_at = item.pub_date.and_then(|d| DateTime::parse_from_rfc2822(&d).ok()).map(|d| d.to_utc());
            p
        })
        .collect())
}

#[derive(Deserialize)]
struct HnSearch {
    hits: Vec<HnHit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HnHit {
    #[serde(rename = "objectID")]
    object_id: String,
}

fn latest_hiring_thread(body: &[u8]) -> Result<String> {
    let search: HnSearch = serde_json::from_slice(body)?;
    Ok(search.hits.into_iter().next().context("no Who is hiring thread found")?.object_id)
}

#[derive(Deserialize)]
struct HnItem {
    #[serde(default)]
    children: Vec<HnComment>,
}

#[derive(Deserialize)]
struct HnComment {
    id: u64,
    text: Option<String>,
    created_at: Option<DateTime<Utc>>,
}

/// Top-level comments in "Who is hiring?" open with `Company | Role | Place | ...`.
fn hacker_news(body: &[u8]) -> Result<Vec<Posting>> {
    let item: HnItem = serde_json::from_slice(body)?;
    Ok(item
        .children
        .into_iter()
        .filter_map(|c| {
            let text = markdown(&c.text?);
            let header = text.lines().next()?.to_string();
            let parts: Vec<&str> = header.split('|').map(str::trim).collect();
            let [company, role, rest @ ..] = parts.as_slice() else {
                return None;
            };
            let url = format!("https://news.ycombinator.com/item?id={}", c.id);
            let mut p = Posting::new("hackernews", c.id, company, role, &url);
            let place = rest.join(" | ");
            p.work_mode = if place.to_lowercase().contains("remote") { WorkMode::Remote } else { WorkMode::Onsite };
            (p.regions, p.countries) = places(&place);
            p.location = place;
            p.description = text;
            p.posted_at = c.created_at;
            Some(p)
        })
        .collect())
}

fn linkedin(body: &[u8]) -> Vec<Posting> {
    let html = Html::parse_fragment(&String::from_utf8_lossy(body));
    let pick = |s: &str| Selector::parse(s).expect("valid selector");
    let (card, title, company, place, link, date) = (
        pick("div.base-card"),
        pick("h3.base-search-card__title"),
        pick("h4.base-search-card__subtitle"),
        pick("span.job-search-card__location"),
        pick("a.base-card__full-link"),
        pick("time"),
    );
    let text = |el: scraper::ElementRef, sel: &Selector| {
        el.select(sel).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_default()
    };
    html.select(&card)
        .filter_map(|el| {
            let urn = el.value().attr("data-entity-urn")?;
            let id = urn.rsplit(':').next()?;
            let href = el.select(&link).next()?.value().attr("href")?;
            let url = href.split('?').next()?.to_string();
            let mut p =
                remote(Posting::new("linkedin", id, &text(el, &company), &text(el, &title), &url), &text(el, &place));
            p.posted_at = el
                .select(&date)
                .next()
                .and_then(|t| t.value().attr("datetime"))
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|d| d.and_utc());
            Some(p)
        })
        .collect()
}
