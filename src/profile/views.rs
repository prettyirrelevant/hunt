use leptos::prelude::*;
use leptos_meta::Title;

use super::{
    api::{ReadReposNow, SaveContext, SaveProfile, You, get_you},
    model::Profile,
};
use crate::ui::parts::Empty;

#[component]
pub fn YouPage() -> impl IntoView {
    let save = ServerAction::<SaveProfile>::new();
    let refresh = ServerAction::<ReadReposNow>::new();
    let data = Resource::new(move || save.version().get(), |_| get_you());
    view! {
        <Title text="You" />
        <h1>"You"</h1>
        <p class="sub">"What hunt knows about you. It learns from your CV, your repos, your sites and what you tell it, with names and contact details removed before any AI reads them."</p>
        <Suspense fallback=|| view! { <p class="sub">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(you) => view! { <Body you save refresh /> }.into_any(),
                    Err(err) => view! { <p class="sub">{err.to_string()}</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn Body(you: You, save: ServerAction<SaveProfile>, refresh: ServerAction<ReadReposNow>) -> impl IntoView {
    let context = ServerAction::<SaveContext>::new();
    let (about, sites) = (RwSignal::new(you.about.clone()), RwSignal::new(you.sites.clone()));
    let profile = you.profile.clone().unwrap_or_default();
    let (headline, summary) = (RwSignal::new(profile.headline.clone()), RwSignal::new(profile.summary.clone()));
    let lines = |v: &[String]| v.join("\n");
    let (roles, skills, avoid) = (
        RwSignal::new(lines(&profile.roles)),
        RwSignal::new(lines(&profile.skills)),
        RwSignal::new(lines(&profile.avoid)),
    );
    let seniority = RwSignal::new(profile.seniority.join(", "));
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
                years: profile.years,
                avoid: split(avoid.get_untracked()),
            },
        });
    };

    view! {
        <section>
            <h2>"Your CV"</h2>
            <form class="drop" method="post" action="/you/cv" enctype="multipart/form-data">
                <p>{match &you.cv {
                    Some(name) => format!("Using {name}. Upload a newer one to replace it."),
                    None => "Upload your CV as a PDF, markdown or text file.".to_string(),
                }}</p>
                <div class="row" style="justify-content:center">
                    <input type="file" name="cv" accept=".pdf,.md,.txt" required />
                    <button class="btn primary" type="submit">"Upload"</button>
                </div>
            </form>
        </section>
        <section>
            <h2>"More about you"</h2>
            <div class="form">
                <div class="field"><label for="about">"In your words"</label><span class="hint">"What your CV and code do not show: what you want next, what you are proud of, what you will not do."</span><textarea id="about" prop:value=about on:input=move |e| about.set(event_target_value(&e))></textarea></div>
                <div class="field"><label for="sites">"Your sites"</label><span class="hint">"Your site, blog, talks or writing, one link per line. A small model reads them each week for work worth showing."</span><textarea id="sites" placeholder="https://yourname.dev" prop:value=sites on:input=move |e| sites.set(event_target_value(&e))></textarea></div>
                <div class="row">
                    <button class="btn primary" on:click=move |_| { context.dispatch(SaveContext { about: about.get_untracked(), sites: sites.get_untracked() }); }>"Save"</button>
                    {move || match context.value().get() {
                        Some(Err(err)) => view! { <span class="chip crit">{err.to_string()}</span> }.into_any(),
                        Some(Ok(())) => view! { <span class="muted">"Saved. hunt reads your links in the background."</span> }.into_any(),
                        None => ().into_any(),
                    }}
                </div>
            </div>
        </section>
        <section>
            <div class="head">
                <h2>"Your profile"</h2>
                {you.profile.is_none().then(|| view! { <span class="muted">"hunt writes this after it reads your CV and repos."</span> })}
            </div>
            <div class="form">
                <div class="field"><label for="headline">"Headline"</label><input id="headline" type="text" prop:value=headline on:input=move |e| headline.set(event_target_value(&e)) /></div>
                <div class="field"><label for="summary">"Summary"</label><textarea id="summary" prop:value=summary on:input=move |e| summary.set(event_target_value(&e))></textarea></div>
                <div class="field"><label for="roles">"Search for"</label><span class="hint">"One role or keyword per line. hunt searches every source for each."</span><textarea id="roles" prop:value=roles on:input=move |e| roles.set(event_target_value(&e))></textarea></div>
                <div class="field"><label for="seniority">"Levels"</label><span class="hint">"Any of: junior, middle, senior, lead."</span><input id="seniority" type="text" prop:value=seniority on:input=move |e| seniority.set(event_target_value(&e)) /></div>
                <div class="field"><label for="skills">"Skills"</label><textarea id="skills" prop:value=skills on:input=move |e| skills.set(event_target_value(&e))></textarea></div>
                <div class="field"><label for="avoid">"Avoid"</label><span class="hint">"Kinds of work or companies you do not want. hunt adds to this as you skip."</span><textarea id="avoid" prop:value=avoid on:input=move |e| avoid.set(event_target_value(&e))></textarea></div>
                <div class="row">
                    <button class="btn primary" on:click=submit>"Save profile"</button>
                    {move || (save.version().get() > 0).then(|| view! { <span class="muted">"Saved. The next search uses it."</span> })}
                </div>
            </div>
        </section>
        <section>
            <div class="head">
                <div>
                    <h2>"Your work"</h2>
                    <p class="muted">{format!("From your sites, and git repositories you committed to under {}.", you.repo_roots.join(", "))}</p>
                </div>
                <button class="btn" on:click=move |_| { refresh.dispatch(ReadReposNow {}); }>
                    {move || if refresh.version().get() > 0 { "Reading your repos…" } else { "Read repos now" }}
                </button>
            </div>
            {if you.work.is_empty() {
                view! { <Empty title="Nothing read yet"><p>"hunt reads your repos and sites weekly, or your repos now if you ask."</p></Empty> }.into_any()
            } else {
                view! {
                    <div class="grid2">
                        {you.work.into_iter().map(|item| view! {
                            <div class="card">
                                <h3>{item.work.as_ref().map_or(item.title.clone(), |w| w.title.clone())}</h3>
                                <p>{item.title.clone()}</p>
                                {match item.work {
                                    Some(w) => view! {
                                        <p style="color:var(--ink)">{w.summary}</p>
                                        <ul class="ev">{w.highlights.into_iter().map(|h| view! { <li>{h}</li> }).collect_view()}</ul>
                                        <div class="chips" style="margin-top:8px">{w.stack.into_iter().map(|s| view! { <span class="chip">{s}</span> }).collect_view()}</div>
                                    }.into_any(),
                                    None => view! { <p>"Waiting to be summarised."</p> }.into_any(),
                                }}
                            </div>
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </section>
    }
}
