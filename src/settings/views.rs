use leptos::prelude::*;
use leptos_meta::Title;

use super::api::{
    Contact, SaveContact, SaveEmail, SaveFolders, SaveProviders, SaveReach, SaveWebModel, Unwatch, View, Watch,
    get_settings,
};

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
    // Refetches after every model check, so each dropdown shows the saved model.
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
        <h1>"Settings"</h1>
        <p class="sub">"The few choices hunt cannot make for you."</p>
        <Suspense fallback=|| view! { <p class="sub">"Loading…"</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(view) => view! { <Body view reach contact providers email folders watch unwatch web /> }.into_any(),
                    Err(err) => view! { <p class="sub">{err.to_string()}</p> }.into_any(),
                }
            })}
        </Suspense>
    }
}

#[component]
fn Saved(version: Signal<usize>, error: Signal<Option<String>>) -> impl IntoView {
    move || match error.get() {
        Some(err) => view! { <span class="chip crit">{err}</span> }.into_any(),
        None => (version.get() > 0).then(|| view! { <span class="muted">"Saved"</span> }).into_any(),
    }
}

fn status<S>(action: ServerAction<S>) -> (Signal<usize>, Signal<Option<String>>)
where
    S: leptos::server_fn::ServerFn<Error = crate::ui::error::Error> + Clone + Send + Sync + 'static,
    S::Output: Clone + Send + Sync + 'static,
{
    let version = Signal::derive(move || action.version().get());
    let error = Signal::derive(move || action.value().get().and_then(Result::err).map(|e| e.to_string()));
    (version, error)
}

#[component]
#[allow(clippy::too_many_arguments)]
fn Body(
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
    let field = move |label: &'static str, get: fn(&Contact) -> String, set: fn(&mut Contact, String)| {
        view! {
            <div class="field">
                <label>{label}</label>
                <input type="text" prop:value=move || person.with(get) on:input=move |e| person.update(|c| set(c, event_target_value(&e))) />
            </div>
        }
    };
    let save_order = move || {
        let names = order.get_untracked().into_iter().filter(|p| p.on).map(|p| p.name).collect();
        providers.dispatch(SaveProviders { order: names });
    };
    let (reach_v, reach_e) = status(reach);
    let (contact_v, contact_e) = status(contact);
    let (email_v, email_e) = status(email);
    let (folders_v, folders_e) = status(folders);
    let (_, watch_e) = status(watch);
    let (web_v, web_e) = status(web);

    view! {
        <section>
            <h2>"Where you can work"</h2>
            <div class="form">
                <div class="field">
                    <label for="country">"You live in"</label>
                    <select id="country" on:change=move |e| country.set(event_target_value(&e))>
                        <option value="" selected=view.country.is_empty()>"Choose a country"</option>
                        {countries().into_iter().map(|(code, name)| {
                            let selected = code == view.country.to_uppercase();
                            view! { <option value=code selected=selected>{name}</option> }
                        }).collect_view()}
                    </select>
                    <span class="hint">"hunt keeps remote jobs open to your country or region, and drops those locked elsewhere."</span>
                </div>
                <label class="toggle"><input type="checkbox" prop:checked=relocate on:change=move |e| relocate.set(event_target_checked(&e)) />"I would move abroad for a job that sponsors a visa or helps with relocation"</label>
                <div class="row">
                    <button class="btn primary" on:click=move |_| { reach.dispatch(SaveReach { country: country.get_untracked(), relocate: relocate.get_untracked() }); }>"Save"</button>
                    <Saved version=reach_v error=reach_e />
                </div>
            </div>
        </section>

        <section>
            <h2>"AI"</h2>
            <p class="muted">"hunt uses the command-line tools you are logged in to, in this order. When one hits a usage limit, it moves to the next. Reading your sites needs little thought, so a small model does it."</p>
            <div class="box">
                {move || order.get().into_iter().enumerate().map(|(i, p)| view! {
                    <div class="row" style="padding:10px 14px;border-bottom:1px solid var(--line)">
                        <label class="toggle"><input type="checkbox" prop:checked=p.on on:change=move |e| { order.update(|o| o[i].on = event_target_checked(&e)); save_order(); } /><b>{p.name.clone()}</b></label>
                        <span class="muted">{p.status.clone()}</span>
                        <span style="margin-left:auto" class="row">
                            <label class="muted" for=format!("web-{}", p.name)>"Reads sites with"</label>
                            <select id=format!("web-{}", p.name) disabled=move || web.pending().get() on:change={
                                let name = p.name.clone();
                                move |e| { web.dispatch(SaveWebModel { provider: name.clone(), model: event_target_value(&e) }); }
                            }>
                                <option value="" selected=p.web_model.is_empty()>"Default"</option>
                                {p.models.iter().map(|m| view! { <option value=m.clone() selected=*m == p.web_model>{m.clone()}</option> }).collect_view()}
                            </select>
                            <button class="btn small" disabled=i == 0 on:click=move |_| { order.update(|o| o.swap(i, i - 1)); save_order(); }>"Up"</button>
                            <button class="btn small" disabled=move || i + 1 == order.with(Vec::len) on:click=move |_| { order.update(|o| o.swap(i, i + 1)); save_order(); }>"Down"</button>
                        </span>
                    </div>
                }).collect_view()}
            </div>
            {move || if web.pending().get() {
                view! { <span class="muted">"Checking the model with one small request…"</span> }.into_any()
            } else {
                view! { <Saved version=web_v error=web_e /> }.into_any()
            }}
        </section>

        <section>
            <h2>"Your details"</h2>
            <p class="muted">"These go on your CV and into application forms. They never go to an AI for writing."</p>
            <div class="form">
                {field("Name", |c| c.name.clone(), |c, v| c.name = v)}
                {field("Email", |c| c.email.clone(), |c, v| c.email = v)}
                {field("Phone", |c| c.phone.clone(), |c, v| c.phone = v)}
                {field("Location", |c| c.location.clone(), |c, v| c.location = v)}
                <div class="field">
                    <label>"Links"</label>
                    <span class="hint">"One per line: GitHub, LinkedIn, portfolio."</span>
                    <textarea prop:value=move || person.with(|c| c.links.clone()) on:input=move |e| person.update(|c| c.links = event_target_value(&e))></textarea>
                </div>
                <div class="row">
                    <button class="btn primary" on:click=move |_| { contact.dispatch(SaveContact { contact: person.get_untracked() }); }>"Save"</button>
                    <Saved version=contact_v error=contact_e />
                </div>
            </div>
        </section>

        <section>
            <h2>"Gmail"</h2>
            <p class="muted">"hunt sends email applications from this account and reads replies to them. Create an app password at myaccount.google.com/apppasswords. It is kept in your macOS Keychain."</p>
            <div class="form">
                <div class="field"><label>"Gmail address"</label><input type="email" prop:value=address on:input=move |e| address.set(event_target_value(&e)) /></div>
                <div class="field">
                    <label>"App password"</label>
                    <input type="password" placeholder=if view.has_password { "Saved in the Keychain. Type to replace it." } else { "16 characters" } prop:value=password on:input=move |e| password.set(event_target_value(&e)) />
                </div>
                <div class="row">
                    <button class="btn primary" on:click=move |_| { email.dispatch(SaveEmail { address: address.get_untracked(), password: password.get_untracked() }); password.set(String::new()); }>"Save"</button>
                    <Saved version=email_v error=email_e />
                </div>
            </div>
        </section>

        <section>
            <h2>"Companies you watch"</h2>
            <p class="muted">"Paste a careers page link. hunt reads that company's job board directly on every search."</p>
            <div class="form">
                <div class="row">
                    <input type="url" placeholder="https://jobs.ashbyhq.com/linear" prop:value=link on:input=move |e| link.set(event_target_value(&e)) style="flex:1" />
                    <button class="btn primary" on:click=move |_| { watch.dispatch(Watch { link: link.get_untracked() }); link.set(String::new()); }>"Watch"</button>
                </div>
                {move || watch_e.get().map(|e| view! { <span class="chip crit">{e}</span> })}
                <div class="chips">
                    {view.watched.into_iter().map(|name| view! {
                        <span class="chip">{name.clone()}" "<button class="btn small" on:click=move |_| { unwatch.dispatch(Unwatch { name: name.clone() }); }>"Remove"</button></span>
                    }).collect_view()}
                </div>
            </div>
        </section>

        <section>
            <h2>"Folders"</h2>
            <div class="form">
                <div class="field"><label>"Backups"</label><span class="hint">"hunt saves a database backup here every night and keeps the last 14."</span><input type="text" prop:value=backup on:input=move |e| backup.set(event_target_value(&e)) /></div>
                <div class="field"><label>"Your code"</label><span class="hint">"hunt looks for git repositories you committed to under these folders, one per line."</span><textarea prop:value=roots on:input=move |e| roots.set(event_target_value(&e))></textarea></div>
                <div class="field"><label>"Never read"</label><span class="hint">"Folders or files to leave out, one per line, like a .gitignore: client-work/, *.secret, ~/Developer/acme"</span><textarea prop:value=ignore on:input=move |e| ignore.set(event_target_value(&e))></textarea></div>
                <div class="row">
                    <button class="btn primary" on:click=move |_| { folders.dispatch(SaveFolders { backup_dir: backup.get_untracked(), repo_roots: roots.get_untracked(), ignore: ignore.get_untracked() }); }>"Save"</button>
                    <Saved version=folders_v error=folders_e />
                </div>
            </div>
        </section>
    }
}

/// ISO names, sorted without a leading "The".
fn countries() -> Vec<(&'static str, &'static str)> {
    let mut all: Vec<_> = celes::Country::get_countries()
        .into_iter()
        .map(|c| (c.alpha2, c.long_name.strip_prefix("The ").unwrap_or(c.long_name)))
        .collect();
    all.sort_by_key(|&(_, name)| name);
    all
}
