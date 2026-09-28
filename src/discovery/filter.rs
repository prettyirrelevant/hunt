use std::sync::LazyLock;

use chrono::Utc;
use regex::Regex;

use super::model::{Reach, Verdict};
use crate::jobs::{Job, WorkMode};

fn phrase(pattern: &str) -> Regex {
    Regex::new(&format!(r"(?i)\b({pattern})\b")).expect("valid pattern")
}

static CLEARANCE: LazyLock<Regex> = LazyLock::new(|| {
    phrase(
        r"security clearance|active clearance|ts/sci|us citizen(ship)?( is)? required|must be a (us|u\.s\.) citizen|green ?card (holder )?required",
    )
});
static VIDEO: LazyLock<Regex> = LazyLock::new(|| {
    phrase(
        r"video (introduction|interview|submission|recording|cover letter)|record (a|yourself|your answers)|one-way video|hirevue|spark ?hire|vidcruiter|willo\.video|loom video",
    )
});
static WORLDWIDE: LazyLock<Regex> = LazyLock::new(|| {
    phrase(
        r"worldwide|anywhere in the world|work from anywhere|remote anywhere|global(ly)? remote|any time ?zone|fully distributed",
    )
});
static LOCKED: LazyLock<Regex> = LazyLock::new(|| {
    phrase(
        r"(us|u\.s\.|usa|united states|canada|uk|united kingdom|eu|europe|emea|latam|apac|india)[- ](only|based)|(must|need to|required to) (be )?(based|located|reside|residing) in|authori[sz]ed to work in the (us|u\.s\.|united states|uk|eu)",
    )
});

pub fn check(job: &Job, reach: &Reach) -> Verdict {
    let requirements = job.requirements.iter().map(|r| r.text.as_str());
    let text = [job.title.as_str(), &job.description].into_iter().chain(requirements).collect::<Vec<_>>().join(" ");

    if CLEARANCE.is_match(&text) && reach.country != "us" {
        return Verdict::Drop("Needs US citizenship or a security clearance".into());
    }
    if let Some(reason) = geography(job, reach, &text) {
        return Verdict::Drop(reason);
    }
    if VIDEO.is_match(&text) {
        return Verdict::Manual("Asks for a recorded video".into());
    }

    let mut flags = vec![];
    if job.posted_at.is_some_and(|at| (Utc::now() - at).num_days() > 30) {
        flags.push("stale");
    }
    if job.salary_min.is_none() && job.salary_max.is_none() {
        flags.push("no_salary");
    }
    Verdict::Keep { flags }
}

fn geography(job: &Job, reach: &Reach, text: &str) -> Option<String> {
    let mine = job.countries.iter().any(|c| c == &reach.country);
    let my_region = job.regions.iter().any(|r| r == "global" || r == reach.region());
    let helps_you_move = job.visa == Some(true) || job.relocation == Some(true);

    match job.work_mode {
        WorkMode::Remote if mine || my_region || WORLDWIDE.is_match(text) => None,
        WorkMode::Remote if !job.regions.is_empty() || !job.countries.is_empty() => {
            Some(format!("Remote only for people in {}", place_list(job)))
        }
        WorkMode::Remote => LOCKED.is_match(text).then(|| "Remote, but locked to one country".into()),
        WorkMode::Onsite | WorkMode::Hybrid if mine || (reach.relocate && helps_you_move) => None,
        WorkMode::Onsite | WorkMode::Hybrid => {
            let place = if job.location.is_empty() { place_list(job) } else { job.location.clone() };
            Some(format!("Office-based in {place} with no visa or relocation mentioned"))
        }
        WorkMode::Unknown => {
            (LOCKED.is_match(text) && !mine && !WORLDWIDE.is_match(text)).then(|| "Locked to one country".into())
        }
    }
}

fn place_list(job: &Job) -> String {
    let names: Vec<String> = job
        .countries
        .iter()
        .map(|c| c.to_uppercase())
        .chain(job.regions.iter().map(|r| region_label(r).to_string()))
        .take(4)
        .collect();
    if names.is_empty() { "another country".into() } else { names.join(", ") }
}

pub fn region_label(region: &str) -> &str {
    match region {
        "global" => "anywhere",
        "north_america" => "North America",
        "latam" => "Latin America",
        "eu" => "the EU",
        "uk" => "the UK",
        "mena" => "the Middle East and North Africa",
        "africa" => "Africa",
        "apac" => "Asia-Pacific",
        "cis" => "the CIS",
        other => other,
    }
}

pub fn region_of(country: &str) -> &'static str {
    const EU: [&str; 30] = [
        "at", "be", "bg", "hr", "cy", "cz", "dk", "ee", "fi", "fr", "de", "gr", "hu", "ie", "it", "lv", "lt", "lu",
        "mt", "nl", "pl", "pt", "ro", "sk", "si", "es", "se", "no", "ch", "is",
    ];
    const MENA: [&str; 17] =
        ["ae", "sa", "qa", "kw", "bh", "om", "jo", "lb", "il", "tr", "eg", "ma", "dz", "tn", "ly", "iq", "ir"];
    const LATAM: [&str; 16] =
        ["mx", "br", "ar", "cl", "co", "pe", "uy", "py", "bo", "ec", "ve", "cr", "pa", "gt", "do", "cu"];
    const APAC: [&str; 18] =
        ["in", "cn", "jp", "kr", "sg", "my", "id", "ph", "th", "vn", "au", "nz", "pk", "bd", "lk", "hk", "tw", "np"];
    const CIS: [&str; 9] = ["ru", "by", "kz", "uz", "am", "az", "ge", "kg", "md"];
    match country {
        "us" | "ca" => "north_america",
        "gb" | "uk" => "uk",
        c if EU.contains(&c) => "eu",
        c if MENA.contains(&c) => "mena",
        c if LATAM.contains(&c) => "latam",
        c if APAC.contains(&c) => "apac",
        c if CIS.contains(&c) => "cis",
        _ => "africa",
    }
}

pub fn places(location: &str) -> (Vec<String>, Vec<String>) {
    const COUNTRIES: [(&str, &str); 14] = [
        ("united states", "us"),
        ("usa", "us"),
        ("us", "us"),
        ("canada", "ca"),
        ("united kingdom", "gb"),
        ("uk", "gb"),
        ("germany", "de"),
        ("netherlands", "nl"),
        ("france", "fr"),
        ("spain", "es"),
        ("portugal", "pt"),
        ("ireland", "ie"),
        ("poland", "pl"),
        ("india", "in"),
    ];
    const REGIONS: [(&str, &[&str]); 9] = [
        ("worldwide", &["global"]),
        ("anywhere", &["global"]),
        ("global", &["global"]),
        ("europe", &["eu"]),
        ("emea", &["eu", "uk", "africa", "mena"]),
        ("africa", &["africa"]),
        ("latam", &["latam"]),
        ("apac", &["apac"]),
        ("north america", &["north_america"]),
    ];

    let text = location.to_lowercase();
    let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).collect();
    let named = |term: &str| {
        if term.contains(' ') { text.contains(term) } else { words.contains(&term) }
    };

    let mut countries: Vec<String> = vec![];
    for (name, code) in COUNTRIES {
        if named(name) && !countries.iter().any(|c| c == code) {
            countries.push(code.into());
        }
    }
    let mut regions: Vec<String> = vec![];
    let from_names = REGIONS.iter().filter(|(name, _)| named(name)).flat_map(|(_, r)| r.iter().copied());
    for region in from_names.chain(countries.iter().map(|c| region_of(c))) {
        if !regions.iter().any(|r| r == region) {
            regions.push(region.into());
        }
    }
    (regions, countries)
}
