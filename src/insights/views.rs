use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;

use super::{
    api::{LOG_PAGE, get_insights, get_log, get_pipeline, get_today},
    model::{Band, Setup, Today},
};
use chrono::Timelike;

use crate::ui::{
    api::SearchNow,
    icons::{Glyph, Icon},
    parts::{Avatar, Empty, PageHead, Pager, StageChip, ago},
};

#[component]
pub fn TodayPage() -> impl IntoView {
    let search = expect_context::<ServerAction<SearchNow>>();
    let data = Resource::new(move || search.version().get(), |_| get_today());
    let now = chrono::Local::now();
    let greeting = match now.hour() {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        _ => "Good evening",
    };
    view! {
        <Title text="Today" />
        <p class="eyebrow">{now.format("%A, %-d %B").to_string()}</p>
        <PageHead title=greeting />
        <Suspense fallback=|| view! { <div class="hero skeleton" style="height:190px"></div> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(today) => view! { <TodayBody today /> }.into_any(),
                    Err(err) => view! { <Empty title="Could not load today"><p>{err.to_string()}</p></Empty> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn TodayBody(today: Today) -> impl IntoView {
    let search = expect_context::<ServerAction<SearchNow>>();
    let searching = move || today.searching || search.pending().get();
    let run = move |_| {
        search.dispatch(SearchNow {});
    };
    let hero = if !today.setup.done() {
        view! { <SetupHero setup=today.setup.clone() /> }.into_any()
    } else if today.jobs == 0 {
        view! {
            <div class="hero">
                <span class="badge"><Icon glyph=Glyph::Search size=22 /></span>
                <div>
                    {move || if searching() {
                        view! {
                            <h2>"Searching for the first time"</h2>
                            <p>"hunt is reading every job board and the companies you watch. Scored jobs appear here in a few minutes."</p>
                            <div class="cta"><span class="state"><i class="pulse on"></i>"Searching every source"</span></div>
                        }.into_any()
                    } else {
                        view! {
                            <h2>"Ready for your first search"</h2>
                            <p>"Your profile is set. hunt searches every three hours on its own, or you can start now."</p>
                            <div class="cta"><button class="btn primary lg" on:click=run><Icon glyph=Glyph::Search size=16 />"Search now"</button></div>
                        }.into_any()
                    }}
                </div>
            </div>
        }.into_any()
    } else if today.ready > 0 {
        let n = today.ready;
        view! {
            <div class="hero">
                <span class="badge"><Icon glyph=Glyph::Review size=22 /></span>
                <div>
                    <h2>{if n == 1 { "One application is ready for you".to_string() } else { format!("{n} applications are ready for you") }}</h2>
                    <p>"Each has a tailored CV, a cover letter and answers. Nothing goes out until you approve it."</p>
                    <div class="cta">
                        <a class="btn primary lg" href="/review">"Review the batch"<Icon glyph=Glyph::Arrow size=16 /></a>
                        <span class="hint">"About a minute each"</span>
                    </div>
                </div>
            </div>
        }.into_any()
    } else {
        view! {
            <div class="hero calm">
                <span class="badge"><Icon glyph=Glyph::Check size=22 /></span>
                <div>
                    <h2>"You are all caught up"</h2>
                    <p>{format!("hunt has {} jobs on file and keeps searching every three hours. New drafts show up here when a job fits.", today.jobs)}</p>
                    <div class="cta">
                        <a class="btn" href="/jobs">"Browse jobs"</a>
                        <button class="btn ghost" disabled=searching on:click=run>{move || if searching() { "Searching…" } else { "Search now" }}</button>
                    </div>
                </div>
            </div>
        }.into_any()
    };
    let todo = [
        (today.replies, "Replies to check", "Emails hunt could not match with confidence.", "/replies", Glyph::Replies),
        (
            today.manual,
            "Good fits to apply to yourself",
            "They ask for something only you can do, such as a video.",
            "/jobs?show=manual",
            Glyph::Jobs,
        ),
    ];
    let waiting: Vec<_> = todo.into_iter().filter(|(n, ..)| *n > 0).collect();
    let stats = [
        (today.seen_24h, "jobs seen today"),
        (today.new_24h, "new to hunt"),
        (today.shortlisted_24h, "shortlisted"),
        (today.applied_7d, "applied this week"),
        (today.heard_back_7d, "replies this week"),
    ];

    view! {
        {hero}
        {(!waiting.is_empty()).then(|| view! {
            <section>
                <h2>"Also waiting for you"</h2>
                <div class="todo">
                    {waiting.into_iter().map(|(n, title, body, href, glyph)| view! {
                        <a href=href>
                            <span class="n num">{n}</span>
                            <span><span class="t">{title}</span><span class="d">{body}</span></span>
                            <Icon glyph />
                        </a>
                    }).collect_view()}
                </div>
            </section>
        })}
        {(today.jobs > 0).then(|| view! {
            <section>
                <h2>"The last day"</h2>
                <div class="stats">
                    {stats.into_iter().map(|(n, label)| view! { <div><b class="num">{n}</b><span>{label}</span></div> }).collect_view()}
                </div>
            </section>
        })}
        {(!today.feed.is_empty()).then(|| view! {
            <section>
                <div class="head"><h2>"What hunt did"</h2><a class="link" href="/log">"Full log"<Icon glyph=Glyph::Arrow size=14 /></a></div>
                <ul class="feed box">
                    {today.feed.into_iter().map(|e| {
                        let body = match e.job_id {
                            Some(id) => view! { <a href=format!("/jobs/{id}")>{e.body}</a> }.into_any(),
                            None => view! { <span>{e.body}</span> }.into_any(),
                        };
                        view! { <li class=e.kind><span>{body}</span><time class="num">{ago(e.at)}</time></li> }
                    }).collect_view()}
                </ul>
            </section>
        })}
    }
}

#[component]
fn SetupHero(setup: Setup) -> impl IntoView {
    let steps = [
        (setup.reach, "Where you can work", "Your country, and whether you would move.", "/settings#where", "Set it"),
        (
            setup.cv,
            "Your CV",
            "Read on this machine. Personal details are removed before any AI sees it.",
            "/you",
            "Add CV",
        ),
        (
            setup.profile,
            "Your profile",
            "hunt writes it from your CV and repos. It decides what to search for.",
            "/you#profile",
            "Review",
        ),
        (setup.ai, "An AI command-line tool", "Log in to claude, codex, opencode or gemini.", "/settings#ai", "Check"),
    ];
    let done = steps.iter().filter(|(ok, ..)| *ok).count();
    view! {
        <div class="hero">
            <span class="badge"><Icon glyph=Glyph::Settings size=22 /></span>
            <div>
                <h2>"Set up hunt"</h2>
                <p>{format!("{done} of 4 done. hunt starts searching as soon as it knows where you can work and what you do.")}</p>
                <div class="progress"><i style=format!("width:{}%", done * 25)></i></div>
                <ol class="checklist">
                    {steps.into_iter().map(|(ok, title, body, href, label)| view! {
                        <li class:done=ok>
                            <span class="tick">{if ok { view! { <Icon glyph=Glyph::Check size=13 /> }.into_any() } else { ().into_any() }}</span>
                            <span><b>{title}</b><span class="d">{body}</span></span>
                            {(!ok).then(|| view! { <a class="btn small" href=href>{label}</a> })}
                        </li>
                    }).collect_view()}
                </ol>
            </div>
        </div>
    }
}

#[component]
pub fn PipelinePage() -> impl IntoView {
    let data = Resource::new(|| (), |()| get_pipeline());
    #[cfg(feature = "hydrate")]
    Effect::new(move || {
        if let Some(Ok(p)) = data.get()
            && !p.flows.is_empty()
        {
            super::charts::draw("sankey", super::charts::sankey(&p.flows));
        }
    });
    view! {
        <Title text="Pipeline" />
        <PageHead title="Pipeline" sub="Every job hunt has seen, from the moment it was found to where it stands now." />
        <section>
            <Suspense>
                {move || Suspend::new(async move {
                    let empty = data.await.map_or(true, |p| p.flows.is_empty());
                    if empty {
                        view! { <Empty title="Nothing to chart yet"><p>"The flow fills in as hunt finds and scores jobs."</p></Empty> }.into_any()
                    } else {
                        view! { <div class="chart" id="sankey"></div> }.into_any()
                    }
                })}
            </Suspense>
        </section>
        <section>
            <h2>"Active applications"</h2>
            <Suspense>
                {move || Suspend::new(async move {
                    let jobs = data.await.map(|p| p.active).unwrap_or_default();
                    if jobs.is_empty() {
                        return view! { <Empty title="No applications yet"><p>"They appear here once you send a batch."</p><a class="btn" href="/review">"Go to review"</a></Empty> }.into_any();
                    }
                    view! {
                        <div class="tablewrap">
                            <table>
                                <thead><tr><th>"Role"</th><th>"Stage"</th><th class="nw">"Since"</th></tr></thead>
                                <tbody>
                                    {jobs.into_iter().map(|job| view! {
                                        <tr>
                                            <td><div class="who"><Avatar name=job.company.clone() /><div><a class="title" href=format!("/jobs/{}", job.id)>{job.title.clone()}</a><div class="co">{job.company.clone()}</div></div></div></td>
                                            <td><StageChip stage=job.stage /></td>
                                            <td class="num nw muted">{ago(job.stage_at)}</td>
                                        </tr>
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                })}
            </Suspense>
        </section>
    }
}

#[component]
pub fn InsightsPage() -> impl IntoView {
    let data = Resource::new(|| (), |()| get_insights());
    #[cfg(feature = "hydrate")]
    Effect::new(move || {
        if let Some(Ok(i)) = data.get() {
            super::charts::draw("weeks", super::charts::weeks(&i.weeks));
        }
    });
    view! {
        <Title text="Insights" />
        <PageHead title="Insights" sub="How your search is going, what works, and what hunt has learned about you." />
        <section class="grid2">
            <div class="card">
                <h3>"Applications and replies"</h3>
                <p>"Per week, last 12 weeks."</p>
                <div class="chart small" id="weeks"></div>
            </div>
            <Suspense>
                {move || Suspend::new(async move {
                    let Ok(i) = data.await else { return ().into_any() };
                    let reply = i.median_reply_days.map_or("No replies yet".to_string(), |d| format!("{d:.0} days"));
                    let model = |m: Option<super::model::Learned>| match m {
                        Some(m) => format!("{} jobs{}", m.examples, m.accuracy.map_or(String::new(), |a| format!(", {:.0}% right on jobs it had not seen", a * 100.0))),
                        None => "Needs about 30 decisions".to_string(),
                    };
                    view! {
                        <div class="card">
                            <h3>"What hunt has learned"</h3>
                            <p>"From your approvals, your skips, and what companies said back."</p>
                            <dl class="kv">
                                <dt>"Decisions"</dt><dd class="num">{i.decisions}</dd>
                                <dt>"Your taste"</dt><dd>{model(i.like)}</dd>
                                <dt>"Reply odds"</dt><dd>{model(i.odds)}</dd>
                                <dt>"First reply"</dt><dd>{reply}" after applying, median"</dd>
                            </dl>
                        </div>
                        <BandCard bands=i.bands />
                        <Bars title="Where your shortlist comes from" note="Jobs shortlisted per source." rows=i.sources.iter().map(|s| (s.source.clone(), s.shortlisted)).collect() />
                        <Bars title="Why jobs were set aside" note="The most common reasons, from rules and scoring." rows=i.drops />
                        <Bars title="Skills you are asked for most" note="In jobs you were shortlisted for, but missing from your profile." rows=i.gaps />
                    }.into_any()
                })}
            </Suspense>
        </section>
    }
}

#[component]
fn BandCard(bands: Vec<Band>) -> impl IntoView {
    view! {
        <div class="card">
            <h3>"Does a high score mean better odds?"</h3>
            <p>"Share of applications that got past the first step, by score. Bands under five applications are too small to read."</p>
            <div class="bars">
                {bands.into_iter().map(|b| {
                    let known = b.decided >= 5;
                    let share = if known { b.moved_on as f64 / b.decided as f64 } else { 0.0 };
                    view! {
                        <div class="bar">
                            <span>{b.band}</span>
                            <span class="track"><span class="fill ok" style=format!("width:{:.0}%", share * 100.0)></span></span>
                            <span class="num muted">{if known { format!("{:.0}%", share * 100.0) } else { format!("n={}", b.decided) }}</span>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
fn Bars(title: &'static str, note: &'static str, rows: Vec<(String, i64)>) -> impl IntoView {
    let top = rows.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1);
    view! {
        <div class="card">
            <h3>{title}</h3>
            <p>{note}</p>
            {if rows.is_empty() {
                view! { <p>"Not enough data yet."</p> }.into_any()
            } else {
                view! {
                    <div class="bars">
                        {rows.into_iter().map(|(label, n)| view! {
                            <div class="bar">
                                <span title=label.clone()>{label.clone()}</span>
                                <span class="track"><span class="fill" style=format!("width:{}%", n * 100 / top)></span></span>
                                <span class="num muted">{n}</span>
                            </div>
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
pub fn LogPage() -> impl IntoView {
    let query = use_query_map();
    let kind = move || query.read().get("kind").unwrap_or_default();
    let page = move || query.read().get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    let data = Resource::new(move || (kind(), page()), |(kind, page)| get_log(kind, page));
    let kinds = [
        ("", "All"),
        ("sweep", "Searches"),
        ("stage", "Moves"),
        ("score", "Scores"),
        ("draft", "Drafts"),
        ("profile", "Profile"),
        ("learn", "Learning"),
        ("system", "System"),
    ];
    view! {
        <Title text="Log" />
        <PageHead title="Log" sub="Everything hunt did, newest first. This is the full record." />
        <div class="filters">
            {kinds.into_iter().map(|(k, label)| view! {
                <a href=format!("/log?kind={k}") attr:aria-current=move || (kind() == k).then_some("true")>{label}</a>
            }).collect_view()}
        </div>
        <Transition>
            {move || Suspend::new(async move {
                let Ok(log) = data.await else { return ().into_any() };
                if log.events.is_empty() {
                    return view! { <Empty title="Nothing here yet"><p>"hunt records every search, score, draft and move here."</p></Empty> }.into_any();
                }
                let kind = kind();
                view! {
                    <div class="box log">
                        {log.events.into_iter().map(|e| {
                            let body = match e.job_id {
                                Some(id) => view! { <a href=format!("/jobs/{id}")>{e.body}</a> }.into_any(),
                                None => view! { <span>{e.body}</span> }.into_any(),
                            };
                            view! { <div><time>{e.at.with_timezone(&chrono::Local).format("%-d %b %H:%M").to_string()}</time><span class=format!("lk {}", e.kind)>{e.kind.clone()}</span>{body}</div> }
                        }).collect_view()}
                    </div>
                    <Pager page=page() per=LOG_PAGE total=log.total link=move |p| format!("/log?kind={kind}&page={p}") />
                }.into_any()
            })}
        </Transition>
    }
}
