use leptos::prelude::*;
use leptos_meta::Title;

use super::{
    api::{DraftView, Item, SaveLetter, SendBatch, UndoSend, get_batch, get_draft},
    model::ChangeKind,
};
use crate::{
    jobs::{Stage, api::Decide},
    ui::{
        parts::{Empty, ScoreRing, StageChip},
        shell::typing,
    },
};

#[component]
pub fn ReviewPage() -> impl IntoView {
    let decide = ServerAction::<Decide>::new();
    let send = ServerAction::<SendBatch>::new();
    let undo = ServerAction::<UndoSend>::new();
    let changed = move || (decide.version().get(), send.version().get(), undo.version().get());
    let batch = Resource::new(changed, |_| get_batch());
    let list = RwSignal::new(Vec::<Item>::new());
    Effect::new(move || {
        if let Some(Ok(items)) = batch.get() {
            list.set(items);
        }
    });
    let selected = RwSignal::new(None::<i64>);
    let current = move || selected.get().or_else(|| list.with(|l| l.first().map(|i| i.id)));
    let tab = RwSignal::new("fit");

    let step = move |by: isize| {
        let items = list.get_untracked();
        let at = current().and_then(|id| items.iter().position(|i| i.id == id)).unwrap_or(0) as isize;
        if let Some(item) = items.get((at + by).clamp(0, items.len().saturating_sub(1) as isize) as usize) {
            selected.set(Some(item.id));
        }
    };
    let call = move |stage: Stage| {
        if let Some(id) = current() {
            decide.dispatch(Decide { id, stage, why: String::new() });
            step(1);
        }
    };
    let keys = window_event_listener(leptos::ev::keydown, move |event| {
        if event.meta_key() || event.ctrl_key() || typing(&event) {
            return;
        }
        match event.key().as_str() {
            "j" => step(1),
            "k" => step(-1),
            "a" => call(Stage::Approved),
            "s" => call(Stage::Skipped),
            "e" => tab.set("letter"),
            _ => {}
        }
    });
    on_cleanup(move || keys.remove());

    view! {
        <Title text="Review" />
        <h1>"Review"</h1>
        <p class="sub">"Approve or skip each application. Nothing goes out until you send the batch, and you can undo for a minute after."</p>
        <Transition fallback=|| view! { <p class="sub">"Loading…"</p> }>
            {move || {
                let items = batch.get().and_then(Result::ok).unwrap_or_default();
                if items.is_empty() {
                    return view! {
                        <Empty title="Nothing to review">
                            <p>"New drafts land here as hunt finds jobs that fit. It checks every few hours."</p>
                        </Empty>
                    }.into_any();
                }
                view! {
                    <div class="review">
                        <div class="queue" role="listbox" aria-label="Applications in this batch">
                            {items.into_iter().map(|item| view! { <QueueRow item selected=Signal::derive(current) on_pick=move |id| selected.set(Some(id)) /> }).collect_view()}
                        </div>
                        <article class="detail">
                            {move || current().map(|id| view! { <Draft id tab decide call /> })}
                        </article>
                    </div>
                }.into_any()
            }}
        </Transition>
        <BatchBar list send undo />
    }
}

#[component]
fn QueueRow(item: Item, selected: Signal<Option<i64>>, on_pick: impl Fn(i64) + 'static) -> impl IntoView {
    let id = item.id;
    view! {
        <button class="qitem" role="option" aria-selected=move || (selected.get() == Some(id)).to_string() on:click=move |_| on_pick(id)>
            <span class="role">{item.title}</span>
            <span class="side-score"><b class="num">{item.score.unwrap_or_default()}</b><StageChip stage=item.stage /></span>
            <span class="co">{format!("{} · {}", item.company, item.place)}</span>
        </button>
    }
}

#[component]
fn Draft(
    id: i64,
    tab: RwSignal<&'static str>,
    decide: ServerAction<Decide>,
    call: impl Fn(Stage) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let data = Resource::new(move || id, get_draft);
    view! {
        <Suspense fallback=|| view! { <p class="muted">"Loading the draft…"</p> }>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(d) => view! { <DraftBody d tab call /> }.into_any(),
                    Err(err) => view! { <p class="muted">{err.to_string()}</p> }.into_any(),
                }
            })}
        </Suspense>
        {move || decide.value().get().and_then(Result::err).map(|e| view! { <p class="chip crit">{e.to_string()}</p> })}
    }
}

#[component]
fn DraftBody(
    d: DraftView,
    tab: RwSignal<&'static str>,
    call: impl Fn(Stage) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let job = d.job;
    let id = job.id;
    let how = match &d.email_to {
        Some(to) => format!("Email to {to}, from your Gmail, with the CV and cover letter attached."),
        None => "Web form. claude fills it in a headless browser and submits it; anything it cannot answer comes back to you.".into(),
    };
    let tabs =
        [("fit", "Fit"), ("cv", "CV changes"), ("letter", "Cover letter"), ("answers", "Answers"), ("files", "Files")];
    let letter = RwSignal::new(d.letter.clone());
    let save = ServerAction::<SaveLetter>::new();
    let assessment = d.assessment.clone();

    view! {
        <div class="dhead">
            <div>
                <h3>{job.title.clone()}</h3>
                <div class="co">{job.company.clone()}</div>
                <div class="meta">
                    <span class="chip">{job.where_label()}</span>
                    {job.salary().map(|s| view! { <span class="chip">{s}</span> })}
                    {(job.visa == Some(true)).then(|| view! { <span class="chip ok">"Visa sponsored"</span> })}
                    {job.flags.iter().map(|f| view! { <span class="chip warn">{crate::jobs::flag_label(f).to_string()}</span> }).collect_view()}
                    <a class="chip" href=job.url.clone() target="_blank" rel="noopener">"Posting ↗"</a>
                </div>
            </div>
            {job.score.map(|score| view! { <ScoreRing score /> })}
        </div>
        {assessment.as_ref().map(|a| view! { <p class="why">{a.why.clone()}</p> })}
        <div class="channel"><b>"How it goes out: "</b>{how}</div>
        <div class="tabs" role="tablist">
            {tabs.into_iter().map(|(key, label)| view! {
                <button role="tab" aria-selected=move || (tab.get() == key).to_string() on:click=move |_| tab.set(key)>{label}</button>
            }).collect_view()}
        </div>
        <div class="pane" role="tabpanel">
            {move || match tab.get() {
                "cv" => view! {
                    <ul class="diff">
                        {d.changes.iter().map(|c| {
                            let (class, sym) = match c.kind { ChangeKind::Add => ("add", "+"), ChangeKind::Move => ("move", "↑"), ChangeKind::Cut => ("cut", "−") };
                            view! { <li class=class><span class="sym">{sym}</span><span>{c.text.clone()}</span></li> }
                        }).collect_view()}
                    </ul>
                    <p class="ev" style="margin-top:12px">"Every line comes from your CV or your repos. Skills hunt could not find there were removed."</p>
                }.into_any(),
                "letter" => view! {
                    <textarea class="letter" prop:value=move || letter.get() on:input=move |e| letter.set(event_target_value(&e)) rows="16"></textarea>
                    <div class="row" style="margin-top:10px">
                        <button class="btn" on:click=move |_| { save.dispatch(SaveLetter { id, letter: letter.get_untracked() }); }>"Save letter"</button>
                        {move || save.version().get().gt(&0).then(|| view! { <span class="muted">"Saved. The PDF is rebuilt."</span> })}
                    </div>
                }.into_any(),
                "answers" => view! {
                    {d.email_subject.clone().map(|s| view! { <dl class="kv"><dt>"Subject"</dt><dd>{s}</dd></dl> })}
                    <dl class="kv">
                        {d.answers.iter().map(|a| view! { <dt>{a.question.clone()}</dt><dd>{a.answer.clone()}</dd> }).collect_view()}
                    </dl>
                }.into_any(),
                "files" => view! {
                    <div class="docs">
                        <a class="btn" href=format!("/documents/{id}/cv.pdf") target="_blank">"CV (PDF)"</a>
                        <a class="btn" href=format!("/documents/{id}/cover-letter.pdf") target="_blank">"Cover letter (PDF)"</a>
                    </div>
                    <p class="ev" style="margin-top:12px">{format!("Written by {}.", d.provider)}</p>
                }.into_any(),
                _ => match assessment.clone() {
                    Some(a) => view! {
                        <div class="cols">
                            <div><h4>"Why it fits"</h4><ul>{a.strengths.into_iter().map(|s| view! { <li>{s.requirement}<span class="ev">{s.evidence}</span></li> }).collect_view()}</ul></div>
                            <div><h4>"Gaps and warnings"</h4><ul>
                                {a.gaps.into_iter().map(|g| view! { <li>{g.requirement}<span class="ev">{g.note}</span></li> }).collect_view()}
                                {a.red_flags.into_iter().map(|f| view! { <li>{f}</li> }).collect_view()}
                            </ul></div>
                        </div>
                    }.into_any(),
                    None => view! { <p class="muted">"No assessment for this job."</p> }.into_any(),
                },
            }}
        </div>
        <div class="decide">
            {match job.stage {
                Stage::Approved => view! { <button class="btn" on:click=move |_| call(Stage::Ready)>"Undo approval"</button> }.into_any(),
                _ => view! {
                    <button class="btn primary" on:click=move |_| call(Stage::Approved)>"Approve"</button>
                    <button class="btn danger" on:click=move |_| call(Stage::Skipped)>"Skip"</button>
                }.into_any(),
            }}
            <span class="hint-keys"><span class="kbd">"A"</span>" approve "<span class="kbd">"S"</span>" skip "<span class="kbd">"E"</span>" edit letter "<span class="kbd">"J"</span><span class="kbd">"K"</span>" move"</span>
        </div>
    }
}

#[component]
fn BatchBar(list: RwSignal<Vec<Item>>, send: ServerAction<SendBatch>, undo: ServerAction<UndoSend>) -> impl IntoView {
    let count = move |stage: Stage| list.with(|l| l.iter().filter(|i| i.stage == stage).count());
    let confirming = RwSignal::new(false);
    let approved = move || list.with(|l| l.iter().filter(|i| i.stage == Stage::Approved).cloned().collect::<Vec<_>>());
    view! {
        <div class="batchbar" hidden=move || list.with(Vec::is_empty)>
            <div class="tally num">
                <span><b>{move || count(Stage::Approved)}</b>" approved"</span>
                <span><b>{move || count(Stage::Ready)}</b>" to review"</span>
                {move || (count(Stage::Sending) > 0).then(|| view! { <span><b>{count(Stage::Sending)}</b>" sending in under a minute"</span> })}
            </div>
            <div class="right">
                {move || (count(Stage::Sending) > 0).then(|| view! { <button class="btn" on:click=move |_| { undo.dispatch(UndoSend {}); }>"Undo send"</button> })}
                <button class="btn primary" disabled=move || count(Stage::Approved) == 0 on:click=move |_| confirming.set(true)>
                    {move || format!("Send {} approved", count(Stage::Approved))}
                </button>
            </div>
            {move || confirming.get().then(|| {
                let items = approved();
                let (email, form): (Vec<_>, Vec<_>) = items.iter().partition(|i| i.by_email);
                let names = |v: &[&Item]| v.iter().map(|i| i.company.clone()).collect::<Vec<_>>().join(", ");
                view! {
                    <div class="confirm">
                        <b>{format!("Send {} applications?", items.len())}</b>
                        <ul>
                            {(!email.is_empty()).then(|| view! { <li>{format!("{} by email from your Gmail: {}", email.len(), names(&email))}</li> })}
                            {(!form.is_empty()).then(|| view! { <li>{format!("{} by web form, filled by claude: {}", form.len(), names(&form))}</li> })}
                        </ul>
                        "They go out in a minute. Until then you can undo."
                        <div class="row" style="margin-top:10px">
                            <button class="btn primary" on:click=move |_| { send.dispatch(SendBatch {}); confirming.set(false); }>"Send"</button>
                            <button class="btn" on:click=move |_| confirming.set(false)>"Cancel"</button>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
