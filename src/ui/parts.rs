use chrono::{DateTime, Local, Utc};
use leptos::prelude::*;

use crate::jobs::Stage;

/// "just now", "12 min ago", "3 h ago", "Tue 09:30", "4 Sep".
pub fn ago(at: DateTime<Utc>) -> String {
    let local = at.with_timezone(&Local);
    match (Utc::now() - at).num_minutes() {
        m if m < 1 => "just now".into(),
        m if m < 60 => format!("{m} min ago"),
        m if m < 60 * 24 => format!("{} h ago", m / 60),
        m if m < 60 * 24 * 6 => local.format("%a %H:%M").to_string(),
        _ => local.format("%-d %b").to_string(),
    }
}

#[component]
pub fn StageChip(stage: Stage) -> impl IntoView {
    view! { <span class=format!("chip {}", stage.tone())>{stage.label()}</span> }
}

#[component]
pub fn ScoreRing(score: i32) -> impl IntoView {
    let radius = 24.0;
    let circumference = 2.0 * std::f64::consts::PI * radius;
    let offset = circumference * (1.0 - f64::from(score.clamp(0, 100)) / 100.0);
    view! {
        <svg class="ring" viewBox="0 0 58 58" role="img" aria-label=format!("Score {score} of 100")>
            <circle cx="29" cy="29" r=radius fill="none" stroke="var(--surface-2)" stroke-width="6" />
            <circle
                cx="29" cy="29" r=radius fill="none" stroke="var(--accent)" stroke-width="6" stroke-linecap="round"
                stroke-dasharray=circumference stroke-dashoffset=offset transform="rotate(-90 29 29)"
            />
            <text x="29" y="34" text-anchor="middle" font-size="15" font-weight="700" fill="var(--ink)">{score}</text>
        </svg>
    }
}

/// `link` builds the URL for a page number.
#[component]
pub fn Pager(page: i64, per: i64, total: i64, link: impl Fn(i64) -> String + 'static) -> impl IntoView {
    let last = ((total + per - 1) / per).max(1);
    let start = (page - 1) * per + 1;
    let label = if total == 0 {
        "Nothing here".to_string()
    } else {
        format!("{start}–{} of {total}", (start + per - 1).min(total))
    };
    let prev = (page > 1).then(|| link(page - 1));
    let next = (page < last).then(|| link(page + 1));
    view! {
        <div class="pager">
            <span class="num">{label}</span>
            <div class="row">
                {prev.map(|href| view! { <a class="btn small" href=href>"Newer"</a> })}
                {next.map(|href| view! { <a class="btn small" href=href>"Older"</a> })}
            </div>
        </div>
    }
}

#[component]
pub fn Empty(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="box empty">
            <b>{title}</b>
            {children()}
        </div>
    }
}
