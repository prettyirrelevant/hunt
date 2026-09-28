use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::{
    NavigateOptions,
    components::Form,
    hooks::{use_navigate, use_params_map, use_query_map},
};

use super::{
    Job, Stage,
    api::{Decide, JOBS_PAGE, SHOWS, get_job, get_jobs},
    flag_label,
};
use crate::ui::{
    parts::{Empty, Pager, ScoreRing, StageChip, ago},
    shell::typing,
};

#[component]
pub fn JobsPage() -> impl IntoView {
    let query = use_query_map();
    let show = move || query.read().get("show").unwrap_or_default();
    let q = move || query.read().get("q").unwrap_or_default();
    let page = move || query.read().get("page").and_then(|p| p.parse().ok()).unwrap_or(1_i64);
    let decide = ServerAction::<Decide>::new();
    let data = Resource::new(move || (show(), q(), page(), decide.version().get()), |(s, q, p, _)| get_jobs(s, q, p));

    let cursor = RwSignal::new(0_usize);
    let rows = RwSignal::new(Vec::<Job>::new());
    Effect::new(move || {
        if let Some(Ok(list)) = data.get() {
            rows.set(list.jobs);
        }
    });
    let navigate = use_navigate();
    let keys = window_event_listener(leptos::ev::keydown, move |event| {
        if event.meta_key() || event.ctrl_key() || typing(&event) {
            return;
        }
        let jobs = rows.get_untracked();
        let Some(job) = jobs.get(cursor.get_untracked()) else { return };
        match event.key().as_str() {
            "j" => cursor.update(|c| *c = (*c + 1).min(jobs.len().saturating_sub(1))),
            "k" => cursor.update(|c| *c = c.saturating_sub(1)),
            "Enter" => navigate(&format!("/jobs/{}", job.id), NavigateOptions::default()),
            "x" => {
                decide.dispatch(Decide { id: job.id, stage: Stage::Skipped, why: String::new() });
            }
            _ => {}
        }
    });
    on_cleanup(move || keys.remove());

    view! {
        <Title text="Jobs" />
        <div class="head">
            <div>
                <h1>"Jobs"</h1>
                <p class="sub">"Everything hunt found, best first. "<span class="kbd">"j"</span>" "<span class="kbd">"k"</span>" to move, "<span class="kbd">"Enter"</span>" to open, "<span class="kbd">"x"</span>" to skip."</p>
            </div>
        </div>
        <div class="filters">
            {SHOWS.into_iter().map(|(key, label)| view! {
                <a href=format!("/jobs?show={key}") attr:aria-current=move || (show() == key).then_some("true")>{label}</a>
            }).collect_view()}
            <Form method="GET" action="/jobs">
                <input type="hidden" name="show" prop:value=show />
                <input type="search" name="q" placeholder="Search titles, companies, skills  /" prop:value=q />
            </Form>
        </div>
        <Transition fallback=|| view! { <p class="sub">"Loading…"</p> }>
            {move || Suspend::new(async move {
                let Ok(list) = data.await else { return ().into_any() };
                if list.jobs.is_empty() {
                    return view! { <Empty title="No jobs here"><p>"Try another filter, or wait for the next search."</p></Empty> }.into_any();
                }
                let (show, q) = (show(), q());
                view! {
                    <div class="tablewrap">
                        <table>
                            <thead><tr><th>"Score"</th><th>"Role"</th><th>"Where"</th><th>"Pay"</th><th>"Stage"</th><th>"Found"</th></tr></thead>
                            <tbody>
                                {list.jobs.into_iter().enumerate().map(|(i, job)| view! { <Row job selected=Signal::derive(move || cursor.get() == i) /> }).collect_view()}
                            </tbody>
                        </table>
                    </div>
                    <Pager page=page() per=JOBS_PAGE total=list.total link=move |p| format!("/jobs?show={show}&q={q}&page={p}") />
                }.into_any()
            })}
        </Transition>
    }
}

#[component]
fn Row(job: Job, selected: Signal<bool>) -> impl IntoView {
    let href = format!("/jobs/{}", job.id);
    let score_class = match job.score {
        Some(s) if s >= 80 => "score hi",
        Some(_) => "score",
        None => "score lo",
    };
    view! {
        <tr class:cursor=selected>
            <td class=score_class>{job.score.map_or("–".to_string(), |s| s.to_string())}</td>
            <td><a class="title" href=href>{job.title.clone()}</a><div class="co">{job.company.clone()}</div></td>
            <td>{job.where_label()}</td>
            <td class="num">{job.salary().unwrap_or_default()}</td>
            <td><StageChip stage=job.stage /></td>
            <td class="num muted">{ago(job.first_seen)}</td>
        </tr>
    }
}

#[component]
pub fn JobPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|id| id.parse::<i64>().ok()).unwrap_or_default();
    let decide = ServerAction::<Decide>::new();
    let data = Resource::new(move || (id(), decide.version().get()), |(id, _)| get_job(id));

    view! {
        <Suspense fallback=|| view! { <p class="sub">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(detail) => view! { <Detail detail decide /> }.into_any(),
                    Err(err) => view! { <h1>"Could not load this job"</h1><p class="sub">{err.to_string()}</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn Detail(detail: super::api::JobDetail, decide: ServerAction<Decide>) -> impl IntoView {
    let job = detail.job;
    let id = job.id;
    let act = move |stage: Stage, why: &'static str| {
        move |_| {
            decide.dispatch(Decide { id, stage, why: why.into() });
        }
    };
    let assessment: Option<crate::matching::model::Assessment> =
        job.assessment.clone().and_then(|a| serde_json::from_value(a).ok());
    let can_shortlist = matches!(job.stage, Stage::New | Stage::Passed | Stage::Filtered | Stage::Skipped);
    let applied = job.stage.is_applied() || job.stage == Stage::Manual;

    view! {
        <Title text=job.title.clone() />
        <div class="dhead">
            <div>
                <h1>{job.title.clone()}</h1>
                <div class="co">{job.company.clone()}</div>
                <div class="meta">
                    <StageChip stage=job.stage />
                    <span class="chip">{job.where_label()}</span>
                    {job.salary().map(|s| view! { <span class="chip">{s}</span> })}
                    {(job.visa == Some(true)).then(|| view! { <span class="chip ok">"Visa sponsored"</span> })}
                    {(job.relocation == Some(true)).then(|| view! { <span class="chip ok">"Relocation help"</span> })}
                    <span class="chip">{format!("{} · {}", job.source, job.posted_at.map_or_else(|| ago(job.first_seen), ago))}</span>
                    {job.flags.iter().map(|f| view! { <span class="chip warn">{flag_label(f).to_string()}</span> }).collect_view()}
                </div>
            </div>
            {job.score.map(|score| view! { <ScoreRing score /> })}
        </div>
        <div class="decide">
            <a class="btn" href=job.url.clone() target="_blank" rel="noopener">"Open posting ↗"</a>
            {can_shortlist.then(|| view! { <button class="btn primary" on:click=act(Stage::Shortlist, "You shortlisted it")>"Draft an application"</button> })}
            {(!job.stage.is_applied() && job.stage != Stage::Skipped).then(|| view! { <button class="btn danger" on:click=act(Stage::Skipped, "")>"Skip"</button> })}
            {(job.stage == Stage::Manual).then(|| view! { <button class="btn" on:click=act(Stage::Applied, "You applied yourself")>"I applied"</button> })}
            {applied.then(|| view! {
                <span class="hint-keys">"Update: "</span>
                <button class="btn small" on:click=act(Stage::Screen, "You updated it")>"Screen"</button>
                <button class="btn small" on:click=act(Stage::Interview, "You updated it")>"Interview"</button>
                <button class="btn small" on:click=act(Stage::Offer, "You updated it")>"Offer"</button>
                <button class="btn small danger" on:click=act(Stage::Rejected, "You updated it")>"Rejected"</button>
            })}
        </div>
        <section class="job">
            <div>
                {assessment.map(|a| view! {
                    <p class="why">{a.why}</p>
                    <div class="cols" style="margin-top:14px">
                        <div><h4>"Why it fits"</h4><ul>{a.strengths.into_iter().map(|s| view! { <li>{s.requirement}<span class="ev">{s.evidence}</span></li> }).collect_view()}</ul></div>
                        <div><h4>"Gaps and warnings"</h4><ul>
                            {a.gaps.into_iter().map(|g| view! { <li>{g.requirement}<span class="ev">{g.note}</span></li> }).collect_view()}
                            {a.red_flags.into_iter().map(|f| view! { <li>{f}</li> }).collect_view()}
                        </ul></div>
                    </div>
                })}
                <h2 style="margin-top:24px">"The posting"</h2>
                <div class="prose" inner_html=detail.html></div>
            </div>
            <aside class="card">
                {(!detail.recording.is_empty()).then(|| view! {
                    <h3>"Recording of the application"</h3>
                    <p>"What the agent saw and typed. Videos are deleted 30 days after an application ends; screenshots stay."</p>
                    {detail.recording.iter().map(|file| {
                        let src = format!("/documents/{id}/recording/{file}");
                        if std::path::Path::new(file).extension().is_some_and(|e| e.eq_ignore_ascii_case("webm")) {
                            view! { <video class="media" controls preload="metadata" src=src></video> }.into_any()
                        } else {
                            let href = src.clone();
                            view! { <a href=href target="_blank"><img class="media" src=src alt="Screenshot from the application" /></a> }.into_any()
                        }
                    }).collect_view()}
                    <hr class="divider" />
                })}
                <h3>"Record"</h3>
                <ul class="timeline">
                    {detail.record.into_iter().map(|e| view! { <li><time>{ago(e.at)}</time>{e.body}</li> }).collect_view()}
                </ul>
            </aside>
        </section>
    }
}
