use leptos::{html, prelude::*};
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
    icons::{Glyph, Icon},
    keys::typing,
    parts::{Avatar, Empty, PageHead, Pager, Score, ScoreRing, StageChip, ago},
    toast::announce,
};

#[component]
pub fn JobsPage() -> impl IntoView {
    let query = use_query_map();
    let show = move || query.read().get("show").unwrap_or_default();
    let q = move || query.read().get("q").unwrap_or_default();
    let page = move || query.read().get("page").and_then(|p| p.parse().ok()).unwrap_or(1_i64);
    let decide = ServerAction::<Decide>::new();
    announce(decide, "Skipped. hunt learns from it.");
    let data = Resource::new(move || (show(), q(), page(), decide.version().get()), |(s, q, p, _)| get_jobs(s, q, p));

    let search = NodeRef::<html::Input>::new();
    Effect::new(move || {
        if query.read().get("focus").is_some()
            && let Some(input) = search.get()
        {
            drop(input.focus());
        }
    });

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
        <PageHead title="Jobs" sub="Everything hunt found, best match first. Press ? for keys." />
        <div class="toolbar">
            <nav class="filters" aria-label="Show">
                {SHOWS.into_iter().map(|(key, label)| view! {
                    <a href=format!("/jobs?show={key}") attr:aria-current=move || (show() == key).then_some("true")>{label}</a>
                }).collect_view()}
            </nav>
            <Form method="GET" action="/jobs" attr:class="searchbox">
                <Icon glyph=Glyph::Search size=15 />
                <input type="hidden" name="show" prop:value=show />
                <input node_ref=search type="search" name="q" placeholder="Search titles, companies, skills" prop:value=q />
            </Form>
        </div>
        <Transition fallback=|| view! { <div class="box skeleton" style="height:360px"></div> }>
            {move || Suspend::new(async move {
                let Ok(list) = data.await else { return ().into_any() };
                if list.jobs.is_empty() {
                    return view! {
                        <Empty title="No jobs match">
                            <p>"Try another filter or a shorter search. hunt searches every source every three hours."</p>
                        </Empty>
                    }.into_any();
                }
                let (show, q) = (show(), q());
                view! {
                    <div class="tablewrap">
                        <table>
                            <thead><tr><th>"Role"</th><th>"Where"</th><th>"Pay"</th><th>"Fit"</th><th>"Stage"</th><th class="nw">"Found"</th></tr></thead>
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
    let place = job.where_label();
    view! {
        <tr class:cursor=selected>
            <td>
                <div class="who">
                    <Avatar name=job.company.clone() />
                    <div><a class="title" href=href>{job.title.clone()}</a><div class="co">{job.company.clone()}</div></div>
                </div>
            </td>
            <td class="clip" title=place.clone()>{place.clone()}</td>
            <td class="num nw">{job.salary().unwrap_or_else(|| "–".into())}</td>
            <td><Score score=job.score /></td>
            <td><StageChip stage=job.stage /></td>
            <td class="num nw muted">{ago(job.first_seen)}</td>
        </tr>
    }
}

#[component]
pub fn JobPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").and_then(|id| id.parse::<i64>().ok()).unwrap_or_default();
    let decide = ServerAction::<Decide>::new();
    announce(decide, "Job updated");
    let data = Resource::new(move || (id(), decide.version().get()), |(id, _)| get_job(id));

    view! {
        <a class="back" href="/jobs"><Icon glyph=Glyph::Arrow size=14 />"Jobs"</a>
        <Transition fallback=|| view! { <div class="box skeleton" style="height:420px"></div> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(detail) => view! { <Detail detail decide /> }.into_any(),
                    Err(err) => view! { <Empty title="Could not load this job"><p>{err.to_string()}</p></Empty> }.into_any(),
                }
            })}
        </Transition>
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
    let found = job.posted_at.map_or_else(|| ago(job.first_seen), ago);

    view! {
        <Title text=job.title.clone() />
        <header class="dhead">
            <div class="who">
                <Avatar name=job.company.clone() large=true />
                <div>
                    <h1>{job.title.clone()}</h1>
                    <div class="co">{format!("{} · {}", job.company, job.where_label())}</div>
                </div>
            </div>
            {job.score.map(|score| view! { <ScoreRing score /> })}
        </header>
        <div class="meta">
            <StageChip stage=job.stage />
            {job.salary().map(|s| view! { <span class="chip outline">{format!("{s} a year")}</span> })}
            {(job.visa == Some(true)).then(|| view! { <span class="chip ok">"Visa sponsored"</span> })}
            {(job.relocation == Some(true)).then(|| view! { <span class="chip ok">"Relocation help"</span> })}
            {job.flags.iter().map(|f| view! { <span class="chip warn">{flag_label(f).to_string()}</span> }).collect_view()}
            <span class="chip">{format!("{} · {found}", job.source)}</span>
        </div>
        <div class="decide">
            {can_shortlist.then(|| view! { <button class="btn primary" on:click=act(Stage::Shortlist, "You shortlisted it")>"Draft an application"</button> })}
            {(job.stage == Stage::Manual).then(|| view! { <button class="btn primary" on:click=act(Stage::Applied, "You applied yourself")><Icon glyph=Glyph::Check size=15 />"I applied"</button> })}
            <a class="btn" href=job.url.clone() target="_blank" rel="noopener">"Open posting ↗"</a>
            {(!job.stage.is_applied() && job.stage != Stage::Skipped).then(|| view! { <button class="btn danger" on:click=act(Stage::Skipped, "")>"Skip"</button> })}
            {applied.then(|| view! {
                <div class="segmented" role="group" aria-label="Where it stands">
                    <span>"Now at"</span>
                    {[(Stage::Screen, "Screen"), (Stage::Interview, "Interview"), (Stage::Offer, "Offer"), (Stage::Rejected, "Rejected")].into_iter().map(|(stage, label)| view! {
                        <button aria-pressed=(job.stage == stage).to_string() on:click=act(stage, "You updated it")>{label}</button>
                    }).collect_view()}
                </div>
            })}
        </div>
        <section class="job">
            <div class="stack">
                {assessment.map(|a| view! {
                    <div class="card read">
                        <h3>"hunt's read"</h3>
                        <p class="why">{a.why}</p>
                        <div class="cols">
                            <div><h4>"Why it fits"</h4><ul>{a.strengths.into_iter().map(|s| view! { <li>{s.requirement}<span class="ev">{s.evidence}</span></li> }).collect_view()}</ul></div>
                            <div class="gaps"><h4>"Gaps and warnings"</h4><ul>
                                {a.gaps.into_iter().map(|g| view! { <li>{g.requirement}<span class="ev">{g.note}</span></li> }).collect_view()}
                                {a.red_flags.into_iter().map(|f| view! { <li>{f}</li> }).collect_view()}
                            </ul></div>
                        </div>
                    </div>
                })}
                <div class="card">
                    <h3>"The posting"</h3>
                    <div class="prose" inner_html=detail.html></div>
                </div>
            </div>
            <aside class="stack">
                {(!detail.recording.is_empty()).then(|| view! {
                    <div class="card">
                        <h3>"Recording"</h3>
                        <p>"What the agent saw and typed. Videos go 30 days after an application ends; screenshots stay."</p>
                        {detail.recording.iter().map(|file| {
                            let src = format!("/documents/{id}/recording/{file}");
                            if std::path::Path::new(file).extension().is_some_and(|e| e.eq_ignore_ascii_case("webm")) {
                                view! { <video class="media" controls preload="metadata" src=src></video> }.into_any()
                            } else {
                                let href = src.clone();
                                view! { <a href=href target="_blank"><img class="media" src=src alt="Screenshot from the application" /></a> }.into_any()
                            }
                        }).collect_view()}
                    </div>
                })}
                <div class="card">
                    <h3>"History"</h3>
                    <ul class="timeline">
                        {detail.record.into_iter().map(|e| view! { <li><time>{ago(e.at)}</time>{e.body}</li> }).collect_view()}
                    </ul>
                </div>
            </aside>
        </section>
    }
}
