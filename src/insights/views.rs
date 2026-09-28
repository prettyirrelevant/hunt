use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;

use super::{
    api::{LOG_PAGE, get_insights, get_log, get_pipeline, get_today},
    model::{Band, Setup, Today},
};
use crate::ui::parts::{Empty, Pager, StageChip, ago};

#[component]
pub fn TodayPage() -> impl IntoView {
    let data = Resource::new(|| (), |()| get_today());
    let heading = chrono::Local::now().format("%A, %-d %B").to_string();
    view! {
        <Title text="Today" />
        <h1>{heading}</h1>
        <Suspense fallback=|| view! { <p class="sub">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(today) => view! { <TodayBody today /> }.into_any(),
                    Err(err) => view! { <p class="sub">{err.to_string()}</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn TodayBody(today: Today) -> impl IntoView {
    let waiting = today.ready + today.replies + today.manual;
    let summary = match waiting {
        0 => "Nothing needs you right now. hunt keeps looking in the background.".to_string(),
        1 => "One thing needs you.".to_string(),
        n => format!("{n} things need you."),
    };
    let setup = today.setup.clone();
    view! {
        <p class="sub">{summary}</p>
        {(!setup.done()).then(|| view! { <SetupSteps setup /> })}
        <section>
            <div class="actions">
                <a class="action" class:calm=today.ready == 0 href="/review">
                    <span class="k num">{today.ready}</span>
                    <span class="t">"Applications to review"</span>
                    <span class="d">"Tailored CVs, letters and answers. Nothing is sent until you approve it."</span>
                    <span class="go">"Review batch →"</span>
                </a>
                <a class="action" class:calm=today.replies == 0 href="/replies">
                    <span class="k num">{today.replies}</span>
                    <span class="t">"Replies to check"</span>
                    <span class="d">"Emails hunt could not match with confidence."</span>
                    <span class="go">"Open replies →"</span>
                </a>
                <a class="action" class:calm=today.manual == 0 href="/jobs?show=manual">
                    <span class="k num">{today.manual}</span>
                    <span class="t">"Good fits you apply to yourself"</span>
                    <span class="d">"They ask for something only you can do, such as a recorded video."</span>
                    <span class="go">"See them →"</span>
                </a>
            </div>
        </section>
        <section>
            <h2>"Last 24 hours"</h2>
            <div class="strip">
                <div><b class="num">{today.seen_24h}</b><span>"jobs seen across every source"</span></div>
                <div><b class="num">{today.new_24h}</b><span>"new to hunt"</span></div>
                <div><b class="num">{today.shortlisted_24h}</b><span>"shortlisted"</span></div>
                <div><b class="num">{today.applied_7d}</b><span>"applications this week"</span></div>
                <div><b class="num">{today.heard_back_7d}</b><span>"replies this week"</span></div>
            </div>
        </section>
        <section>
            <h2>"Recently"</h2>
            {if today.feed.is_empty() {
                view! { <Empty title="Quiet so far"><p>"The first search runs a minute after setup."</p></Empty> }.into_any()
            } else {
                view! {
                    <ul class="feed">
                        {today.feed.into_iter().map(|e| view! {
                            <li><time class="num mono">{ago(e.at)}</time><span>{e.body}</span></li>
                        }).collect_view()}
                    </ul>
                }.into_any()
            }}
        </section>
    }
}

#[component]
fn SetupSteps(setup: Setup) -> impl IntoView {
    let step = |n: u8, done: bool, title: &'static str, body: &'static str, href: &'static str, label: &'static str| {
        view! {
            <div class="step" class:done=done>
                <span class="n">{if done { "✓".to_string() } else { n.to_string() }}</span>
                <div>
                    <h3>{title}</h3>
                    <p class="muted">{body}</p>
                    {(!done).then(|| view! { <a class="btn small" href=href>{label}</a> })}
                </div>
            </div>
        }
    };
    view! {
        <section>
            <h2>"Finish setting up"</h2>
            <div class="steps">
                {step(1, setup.reach, "Where you can work", "Your country, and whether you would move for the right role.", "/settings", "Set it")}
                {step(2, setup.cv, "Your CV", "hunt reads it on this machine and strips personal details before any AI sees it.", "/you", "Add your CV")}
                {step(3, setup.profile, "Your profile", "hunt writes it from your CV and your repos. It decides what to search for.", "/you", "See your profile")}
                {step(4, setup.ai, "An AI command-line tool", "Install and log in to claude, codex, opencode or gemini.", "/settings", "Check AI")}
            </div>
        </section>
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
        <h1>"Pipeline"</h1>
        <p class="sub">"Every job hunt has seen, from the moment it was found to where it stands now."</p>
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
                        return view! { <Empty title="No applications yet"><p>"They appear here once you send a batch."</p></Empty> }.into_any();
                    }
                    view! {
                        <div class="tablewrap">
                            <table>
                                <thead><tr><th>"Role"</th><th>"Stage"</th><th>"Since"</th></tr></thead>
                                <tbody>
                                    {jobs.into_iter().map(|job| view! {
                                        <tr>
                                            <td><a class="title" href=format!("/jobs/{}", job.id)>{job.title.clone()}</a><div class="co">{job.company.clone()}</div></td>
                                            <td><StageChip stage=job.stage /></td>
                                            <td class="num">{ago(job.stage_at)}</td>
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
        <h1>"Insights"</h1>
        <p class="sub">"How your search is going, what works, and what hunt has learned about you."</p>
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
        <h1>"Log"</h1>
        <p class="sub">"Everything hunt did, newest first. This is the full record."</p>
        <div class="filters">
            {kinds.into_iter().map(|(k, label)| view! {
                <a href=format!("/log?kind={k}") attr:aria-current=move || (kind() == k).then_some("true")>{label}</a>
            }).collect_view()}
        </div>
        <Transition>
            {move || Suspend::new(async move {
                let Ok(log) = data.await else { return ().into_any() };
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
