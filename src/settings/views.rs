use leptos::prelude::*;
use leptos_meta::Title;

use super::api::{
    Contact, SaveContact, SaveEmail, SaveFolders, SaveProviders, SaveReach, SaveWebModel, Unwatch, View, Watch,
    get_settings,
};
use crate::ui::{
    icons::{Glyph, Icon},
    parts::{Empty, PageHead},
    toast::announce,
};

const SECTIONS: [(&str, &str); 6] = [
    ("where", "Where you work"),
    ("details", "Your details"),
    ("ai", "AI"),
    ("gmail", "Gmail"),
    ("companies", "Companies"),
    ("folders", "Folders"),
];

#[component]
pub fn SettingsPage() -> impl IntoView {
    let reach = ServerAction::<SaveReach>::new();
    let contact = ServerAction::<SaveContact>::new();
    let providers = ServerAction::<SaveProviders>::new();
    let email = ServerAction::<SaveEmail>::new();
    let folders = ServerAction::<SaveFolders>::new();
    let watch = ServerAction::<Watch>::new();
    let unwatch = ServerAction::<Unwatch>::new();
    let web = ServerAction::<SaveWebModel>::new();
    announce(reach, "Saved where you can work");
    announce(contact, "Saved your details");
    announce(providers, "Saved the AI order");
    announce(email, "Saved Gmail");
    announce(folders, "Saved folders");
    announce(watch, "hunt now watches that company");
    announce(unwatch, "Stopped watching");
    announce(web, "Model checked and saved");
    let changed = move || {
        (
            providers.version().get(),
            email.version().get(),
            watch.version().get(),
            unwatch.version().get(),
            web.version().get(),
        )
    };
    let data = Resource::new(changed, |_| get_settings());
    view! {
        <Title text="Settings" />
        <PageHead title="Settings" sub="The few choices hunt cannot make for you." />
        <div class="settings">
            <nav class="toc" aria-label="Sections">
                {SECTIONS.into_iter().map(|(id, label)| view! { <a href=format!("#{id}")>{label}</a> }).collect_view()}
            </nav>
            <Transition fallback=|| view! { <div class="panel skeleton" style="height:480px"></div> }>
                {move || Suspend::new(async move {
                    match data.await {
                        Ok(view) => view! { <Panels view reach contact providers email folders watch unwatch web /> }.into_any(),
                        Err(err) => view! { <Empty title="Could not load settings"><p>{err.to_string()}</p></Empty> }.into_any(),
                    }
                })}
            </Transition>
        </div>
    }
}

#[component]
fn Panel(id: &'static str, title: &'static str, #[prop(into)] note: String, children: Children) -> impl IntoView {
    view! {
        <section class="panel" id=id>
            <div class="panel-head"><h3>{title}</h3><p>{note}</p></div>
            {children()}
        </section>
    }
}

#[component]
fn Save(pending: Signal<bool>, on_save: impl Fn() + 'static, #[prop(optional)] note: &'static str) -> impl IntoView {
    view! {
        <div class="panel-foot">
            {(!note.is_empty()).then(|| view! { <span class="note">{note}</span> })}
            <button class="btn primary" disabled=pending on:click=move |_| on_save()>
                {move || if pending.get() { "Saving…" } else { "Save" }}
            </button>
        </div>
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn Panels(
    view: View,
    reach: ServerAction<SaveReach>,
    contact: ServerAction<SaveContact>,
    providers: ServerAction<SaveProviders>,
    email: ServerAction<SaveEmail>,
    folders: ServerAction<SaveFolders>,
    watch: ServerAction<Watch>,
    unwatch: ServerAction<Unwatch>,
    web: ServerAction<SaveWebModel>,
) -> impl IntoView {
    let country = RwSignal::new(view.country.to_uppercase());
    let relocate = RwSignal::new(view.relocate);
    let person = RwSignal::new(view.contact.clone());
    let order = RwSignal::new(view.providers.clone());
    let (address, password) = (RwSignal::new(view.email.clone()), RwSignal::new(String::new()));
    let (backup, roots) = (RwSignal::new(view.backup_dir.clone()), RwSignal::new(view.repo_roots.clone()));
    let ignore = RwSignal::new(view.ignore.clone());
    let link = RwSignal::new(String::new());
    let field = move |label: &'static str,
                      kind: &'static str,
                      get: fn(&Contact) -> String,
                      set: fn(&mut Contact, String)| {
        view! {
            <label class="field">
                <span class="label">{label}</span>
                <input type=kind prop:value=move || person.with(get) on:input=move |e| person.update(|c| set(c, event_target_value(&e))) />
            </label>
        }
    };
    let save_order = move || {
        let names = order.get_untracked().into_iter().filter(|p| p.on).map(|p| p.name).collect();
        providers.dispatch(SaveProviders { order: names });
    };
    let add_company = move || {
        if !link.get_untracked().trim().is_empty() {
            watch.dispatch(Watch { link: link.get_untracked() });
            link.set(String::new());
        }
    };
    let pending = |pending: Memo<bool>| Signal::derive(move || pending.get());
    let connected = view.has_password && !view.email.is_empty();

    view! {
        <div class="panels">
            <Panel id="where" title="Where you can work" note="hunt keeps remote jobs open to your country or region, and drops those locked elsewhere.">
                <div class="panel-body">
                    <label class="field">
                        <span class="label">"You live in"</span>
                        <select on:change=move |e| country.set(event_target_value(&e))>
                            <option value="" selected=view.country.is_empty()>"Choose a country"</option>
                            {countries().into_iter().map(|(code, name)| {
                                let selected = code == view.country.to_uppercase();
                                view! { <option value=code selected=selected>{name}</option> }
                            }).collect_view()}
                        </select>
                    </label>
                    <label class="switch">
                        <input type="checkbox" prop:checked=relocate on:change=move |e| relocate.set(event_target_checked(&e)) />
                        <span class="track"></span>
                        <span>"I would move abroad for a job that sponsors a visa or helps with relocation"</span>
                    </label>
                </div>
                <Save pending=pending(reach.pending()) on_save=move || { reach.dispatch(SaveReach { country: country.get_untracked(), relocate: relocate.get_untracked() }); } />
            </Panel>

            <Panel id="details" title="Your details" note="These go on your CV and into application forms. They never go to an AI for writing.">
                <div class="panel-body">
                    <div class="grid-2">
                        {field("Name", "text", |c| c.name.clone(), |c, v| c.name = v)}
                        {field("Email", "email", |c| c.email.clone(), |c, v| c.email = v)}
                        {field("Phone", "text", |c| c.phone.clone(), |c, v| c.phone = v)}
                        {field("City and country", "text", |c| c.location.clone(), |c, v| c.location = v)}
                    </div>
                    <label class="field">
                        <span class="label">"Links"</span>
                        <textarea rows="3" placeholder="https://github.com/you" prop:value=move || person.with(|c| c.links.clone()) on:input=move |e| person.update(|c| c.links = event_target_value(&e))></textarea>
                        <span class="hint">"One per line: GitHub, LinkedIn, portfolio."</span>
                    </label>
                </div>
                <Save pending=pending(contact.pending()) on_save=move || { contact.dispatch(SaveContact { contact: person.get_untracked() }); } />
            </Panel>

            <Panel id="ai" title="AI" note="hunt uses the command-line tools you are logged in to, top first. When one hits a usage limit, it moves to the next. A small model is enough to read your sites.">
                <div class="panel-body">
                    <div class="providers">
                        {move || {
                            let count = order.with(Vec::len);
                            order.get().into_iter().enumerate().map(|(i, p)| {
                                let ready = p.status == "ready";
                                let name = p.name.clone();
                                view! {
                                    <div class="provider" class:off=!p.on>
                                        <span class="rank num">{i + 1}</span>
                                        <label class="switch" title="Use this tool">
                                            <input type="checkbox" prop:checked=p.on disabled=p.status == "not installed" on:change=move |e| { order.update(|o| o[i].on = event_target_checked(&e)); save_order(); } />
                                            <span class="track"></span>
                                        </label>
                                        <div>
                                            <span class="name">{p.name.clone()}</span>" "
                                            <span class=if ready { "chip ok" } else { "chip" }>{p.status.clone()}</span>
                                        </div>
                                        <label class="model">
                                            "Sites"
                                            <select class="small" disabled=move || web.pending().get() on:change=move |e| { web.dispatch(SaveWebModel { provider: name.clone(), model: event_target_value(&e) }); }>
                                                <option value="" selected=p.web_model.is_empty()>"Default model"</option>
                                                {p.models.iter().map(|m| view! { <option value=m.clone() selected=*m == p.web_model>{m.clone()}</option> }).collect_view()}
                                            </select>
                                        </label>
                                        <span class="order">
                                            <button class="btn ghost icon-only" aria-label="Move up" disabled=i == 0 on:click=move |_| { order.update(|o| o.swap(i, i - 1)); save_order(); }><Icon glyph=Glyph::Up /></button>
                                            <button class="btn ghost icon-only" aria-label="Move down" disabled=i + 1 == count on:click=move |_| { order.update(|o| o.swap(i, i + 1)); save_order(); }><Icon glyph=Glyph::Down /></button>
                                        </span>
                                    </div>
                                }
                            }).collect_view()
                        }}
                    </div>
                    {move || web.pending().get().then(|| view! { <span class="hint"><i class="pulse on"></i>" Checking the model with one small request…"</span> })}
                </div>
            </Panel>

            <Panel id="gmail" title="Gmail" note="hunt sends email applications from this account and reads replies to them. The app password stays in your Keychain.">
                <div class="panel-body">
                    <div class="grid-2">
                        <label class="field">
                            <span class="label">"Gmail address"</span>
                            <input type="email" placeholder="you@gmail.com" prop:value=address on:input=move |e| address.set(event_target_value(&e)) />
                        </label>
                        <label class="field">
                            <span class="label">"App password"</span>
                            <input type="password" placeholder=if view.has_password { "Saved. Type to replace it." } else { "16 characters" } prop:value=password on:input=move |e| password.set(event_target_value(&e)) />
                        </label>
                    </div>
                    <span class="hint">"Create one at "<a href="https://myaccount.google.com/apppasswords" target="_blank" rel="noopener">"myaccount.google.com/apppasswords"</a>". Your normal password does not work here."</span>
                </div>
                <div class="panel-foot">
                    <span class="note">{if connected { view! { <span class="chip ok">"Connected"</span> }.into_any() } else { view! { <span>"Not connected"</span> }.into_any() }}</span>
                    <button class="btn primary" disabled=move || email.pending().get() on:click=move |_| { email.dispatch(SaveEmail { address: address.get_untracked(), password: password.get_untracked() }); password.set(String::new()); }>
                        {move || if email.pending().get() { "Saving…" } else { "Save" }}
                    </button>
                </div>
            </Panel>

            <Panel id="companies" title="Companies you watch" note="Paste a careers page. hunt reads that company's job board directly on every search.">
                <div class="panel-body">
                    <form class="row" on:submit=move |e| { e.prevent_default(); add_company(); }>
                        <input type="url" placeholder="https://jobs.ashbyhq.com/linear" prop:value=link on:input=move |e| link.set(event_target_value(&e)) style="flex:1;width:auto" />
                        <button class="btn primary" type="submit" disabled=move || watch.pending().get()>"Watch"</button>
                    </form>
                    {if view.watched.is_empty() {
                        view! { <span class="hint">"Works with Greenhouse, Lever, Ashby, Workable, SmartRecruiters, Recruitee, Rippling, Breezy and Workday."</span> }.into_any()
                    } else {
                        view! {
                            <div class="watchlist">
                                {view.watched.into_iter().map(|name| view! {
                                    <div>
                                        <crate::ui::parts::Avatar name=name.split(':').next_back().unwrap_or(&name).to_string() />
                                        <span>{name.clone()}</span>
                                        <button class="btn ghost small" on:click=move |_| { unwatch.dispatch(Unwatch { name: name.clone() }); }>"Remove"</button>
                                    </div>
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </div>
            </Panel>

            <Panel id="folders" title="Folders" note="Where hunt keeps backups and where it looks for your code.">
                <div class="panel-body">
                    <label class="field">
                        <span class="label">"Backups"</span>
                        <input type="text" class="mono" prop:value=backup on:input=move |e| backup.set(event_target_value(&e)) />
                        <span class="hint">"A database backup every night. hunt keeps the last 14."</span>
                    </label>
                    <label class="field">
                        <span class="label">"Your code"</span>
                        <textarea rows="3" class="mono" placeholder="~/Developer" prop:value=roots on:input=move |e| roots.set(event_target_value(&e))></textarea>
                        <span class="hint">"Folders with git repositories you committed to, one per line."</span>
                    </label>
                    <label class="field">
                        <span class="label">"Never read"</span>
                        <textarea rows="3" class="mono" placeholder="client-work/" prop:value=ignore on:input=move |e| ignore.set(event_target_value(&e))></textarea>
                        <span class="hint">"Like a .gitignore: client-work/, *.secret, ~/Developer/acme"</span>
                    </label>
                </div>
                <Save pending=pending(folders.pending()) on_save=move || { folders.dispatch(SaveFolders { backup_dir: backup.get_untracked(), repo_roots: roots.get_untracked(), ignore: ignore.get_untracked() }); } />
            </Panel>
        </div>
    }
}

fn countries() -> Vec<(&'static str, &'static str)> {
    let mut all: Vec<_> = celes::Country::get_countries()
        .into_iter()
        .map(|c| (c.alpha2, c.long_name.strip_prefix("The ").unwrap_or(c.long_name)))
        .collect();
    all.sort_by_key(|&(_, name)| name);
    all
}
