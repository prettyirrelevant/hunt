use leptos::{html, prelude::*};
use leptos_meta::Title;
use leptos_router::{
    NavigateOptions,
    hooks::{use_navigate, use_query_map},
};

use super::{
    api::{ReadReposNow, SaveContext, SaveProfile, You, get_you},
    model::Profile,
};
use crate::ui::{
    icons::{Glyph, Icon},
    parts::{Empty, PageHead},
    toast::{Toasts, Tone, announce},
};

#[component]
pub fn YouPage() -> impl IntoView {
    let save = ServerAction::<SaveProfile>::new();
    let refresh = ServerAction::<ReadReposNow>::new();
    let context = ServerAction::<SaveContext>::new();
    announce(save, "Profile saved. The next search uses it.");
    announce(refresh, "Reading your repos in the background");
    announce(context, "Saved. hunt reads your links in the background.");
    let data = Resource::new(move || save.version().get(), |_| get_you());

    let query = use_query_map();
    let toasts = expect_context::<Toasts>();
    let navigate = use_navigate();
    Effect::new(move || {
        let (imported, failed) = query.with(|q| (q.get("cv").is_some(), q.get("cv_error")));
        if imported {
            toasts.show("CV imported. hunt is rewriting your profile.", Tone::Done);
        } else if let Some(why) = failed {
            toasts.show(format!("Could not read that CV: {why}"), Tone::Failed);
        } else {
            return;
        }
        navigate("/you", NavigateOptions { replace: true, ..Default::default() });
    });

    view! {
        <Title text="You" />
        <PageHead title="You" sub="What hunt knows about you. Names and contact details are removed before any AI reads it." />
        <Transition fallback=|| view! { <div class="panel skeleton" style="height:420px"></div> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(you) => view! { <Body you save refresh context /> }.into_any(),
                    Err(err) => view! { <Empty title="Could not load your profile"><p>{err.to_string()}</p></Empty> }.into_any(),
                }
            })}
        </Transition>
    }
}

#[component]
fn Body(
    you: You,
    save: ServerAction<SaveProfile>,
    refresh: ServerAction<ReadReposNow>,
    context: ServerAction<SaveContext>,
) -> impl IntoView {
    let (about, sites) = (RwSignal::new(you.about.clone()), RwSignal::new(you.sites.clone()));
    let has_profile = you.profile.is_some();
    let profile = you.profile.clone().unwrap_or_default();
    let reading = move || refresh.pending().get() || refresh.version().get() > 0;
    let roots = you.repo_roots.join(", ");

    view! {
        <div class="stack gap-lg">
            <Upload cv=you.cv.clone() />

            <section class="panel" id="profile">
                <div class="panel-head">
                    <h3>"Your profile"</h3>
                    <p>{if has_profile { "hunt wrote this from your CV and your work. It decides what hunt searches for and how it scores jobs." } else { "hunt writes this after it reads your CV and your repos." }}</p>
                </div>
                <ProfileEditor profile save />
            </section>

            <section class="panel">
                <div class="panel-head">
                    <h3>"Tell hunt more"</h3>
                    <p>"What your CV and code do not show. hunt weighs this as your own view."</p>
                </div>
                <div class="panel-body">
                    <label class="field">
                        <span class="label">"In your words"</span>
                        <textarea rows="4" placeholder="I want to work on payments infrastructure. I will not do crypto trading." prop:value=about on:input=move |e| about.set(event_target_value(&e))></textarea>
                    </label>
                    <label class="field">
                        <span class="label">"Your sites"</span>
                        <textarea rows="2" class="mono" placeholder="https://yourname.dev" prop:value=sites on:input=move |e| sites.set(event_target_value(&e))></textarea>
                        <span class="hint">"Your site, blog or talks, one per line. A small model reads them each week for work worth showing."</span>
                    </label>
                </div>
                <div class="panel-foot">
                    <button class="btn primary" disabled=move || context.pending().get() on:click=move |_| { context.dispatch(SaveContext { about: about.get_untracked(), sites: sites.get_untracked() }); }>
                        {move || if context.pending().get() { "Saving…" } else { "Save" }}
                    </button>
                </div>
            </section>

            <section>
                <div class="head">
                    <div>
                        <h2>"Your work"</h2>
                        <p class="hint">{format!("From your sites, and git repositories you committed to under {roots}.")}</p>
                    </div>
                    <button class="btn" disabled=reading on:click=move |_| { refresh.dispatch(ReadReposNow {}); }>
                        {move || if reading() { "Reading your repos…" } else { "Read repos now" }}
                    </button>
                </div>
                {if you.work.is_empty() {
                    view! { <Empty title="Nothing read yet"><p>"hunt reads your repos and sites every week, or your repos now if you ask."</p></Empty> }.into_any()
                } else {
                    view! {
                        <div class="work">
                            {you.work.into_iter().map(|item| view! {
                                <article class="card">
                                    <h3>{item.work.as_ref().map_or(item.title.clone(), |w| w.title.clone())}</h3>
                                    <p class="src" title=item.title.clone()>{item.title.clone()}</p>
                                    {match item.work {
                                        Some(w) => view! {
                                            <p class="sum">{w.summary}</p>
                                            <ul>{w.highlights.into_iter().map(|h| view! { <li>{h}</li> }).collect_view()}</ul>
                                            <div class="chips">{w.stack.into_iter().map(|s| view! { <span class="chip">{s}</span> }).collect_view()}</div>
                                        }.into_any(),
                                        None => view! { <p class="sum muted">"Waiting to be summarised."</p> }.into_any(),
                                    }}
                                </article>
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
            </section>
        </div>
    }
}

/// Picking or dropping a file uploads it at once.
#[component]
fn Upload(cv: Option<String>) -> impl IntoView {
    let form = NodeRef::<html::Form>::new();
    let sending = RwSignal::new(false);
    let has_cv = cv.is_some();
    view! {
        <form node_ref=form class="upload" class:busy=sending method="post" action="/you/cv" enctype="multipart/form-data">
            <input type="file" name="cv" accept=".pdf,.md,.txt" aria-label="Upload your CV" on:change=move |_| {
                if let Some(form) = form.get() {
                    sending.set(true);
                    drop(form.submit());
                }
            } />
            <span class="file">"CV"</span>
            <span>
                <b>{move || if sending.get() { "Reading your CV…".to_string() } else { cv.clone().unwrap_or_else(|| "Add your CV".into()) }}</b>
                <span class="hint">{if has_cv { "Drop a newer file here to replace it." } else { "Drop a PDF, Markdown or text file here, or click to choose one." }}</span>
            </span>
            <span class="btn"><Icon glyph=Glyph::Upload size=15 />{if has_cv { "Replace" } else { "Choose file" }}</span>
        </form>
    }
}

#[component]
fn ProfileEditor(profile: Profile, save: ServerAction<SaveProfile>) -> impl IntoView {
    let editing = RwSignal::new(false);
    let lines = |v: &[String]| v.join("\n");
    let headline = RwSignal::new(profile.headline.clone());
    let summary = RwSignal::new(profile.summary.clone());
    let roles = RwSignal::new(lines(&profile.roles));
    let skills = RwSignal::new(lines(&profile.skills));
    let avoid = RwSignal::new(lines(&profile.avoid));
    let seniority = RwSignal::new(profile.seniority.join(", "));
    let years = profile.years;
    let split =
        |s: String| s.split(['\n', ',']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>();
    let submit = move |_| {
        save.dispatch(SaveProfile {
            profile: Profile {
                headline: headline.get_untracked(),
                summary: summary.get_untracked(),
                roles: split(roles.get_untracked()),
                seniority: split(seniority.get_untracked()),
                skills: split(skills.get_untracked()),
                years,
                avoid: split(avoid.get_untracked()),
            },
        });
        editing.set(false);
    };
    let chips = move |items: Vec<String>, empty: &'static str| {
        if items.is_empty() {
            view! { <span class="muted">{empty}</span> }.into_any()
        } else {
            view! { <div class="chips">{items.into_iter().map(|i| view! { <span class="chip outline">{i}</span> }).collect_view()}</div> }.into_any()
        }
    };

    view! {
        {move || if editing.get() {
            view! {
                <div class="panel-body">
                    <label class="field"><span class="label">"Headline"</span><input type="text" prop:value=headline on:input=move |e| headline.set(event_target_value(&e)) /></label>
                    <label class="field"><span class="label">"Summary"</span><textarea rows="4" prop:value=summary on:input=move |e| summary.set(event_target_value(&e))></textarea></label>
                    <div class="grid-2">
                        <label class="field"><span class="label">"Search for"</span><textarea rows="5" prop:value=roles on:input=move |e| roles.set(event_target_value(&e))></textarea><span class="hint">"One role or keyword per line."</span></label>
                        <label class="field"><span class="label">"Skills"</span><textarea rows="5" prop:value=skills on:input=move |e| skills.set(event_target_value(&e))></textarea><span class="hint">"One per line."</span></label>
                        <label class="field"><span class="label">"Levels"</span><input type="text" prop:value=seniority on:input=move |e| seniority.set(event_target_value(&e)) /><span class="hint">"Any of: junior, middle, senior, lead."</span></label>
                        <label class="field"><span class="label">"Avoid"</span><textarea rows="3" prop:value=avoid on:input=move |e| avoid.set(event_target_value(&e))></textarea><span class="hint">"hunt adds to this as you skip."</span></label>
                    </div>
                </div>
                <div class="panel-foot">
                    <button class="btn ghost" on:click=move |_| editing.set(false)>"Cancel"</button>
                    <button class="btn primary" on:click=submit>"Save profile"</button>
                </div>
            }.into_any()
        } else {
            let split_now = |s: String| s.split(['\n', ',']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>();
            view! {
                <div class="panel-body profile-view">
                    <div>
                        <div class="headline">{move || { let h = headline.get(); if h.is_empty() { "No headline yet".to_string() } else { h } }}</div>
                        <p style="margin-top:6px">{move || summary.get()}</p>
                    </div>
                    <dl class="facts">
                        <dt>"Searches for"</dt><dd>{move || chips(split_now(roles.get()), "Nothing yet")}</dd>
                        <dt>"Levels"</dt><dd>{move || chips(split_now(seniority.get()), "Any level")}</dd>
                        <dt>"Skills"</dt><dd>{move || chips(split_now(skills.get()), "None found yet")}</dd>
                        <dt>"Avoids"</dt><dd>{move || chips(split_now(avoid.get()), "Nothing yet. hunt learns this from your skips.")}</dd>
                    </dl>
                </div>
                <div class="panel-foot">
                    <button class="btn" on:click=move |_| editing.set(true)>"Edit profile"</button>
                </div>
            }.into_any()
        }}
    }
}
