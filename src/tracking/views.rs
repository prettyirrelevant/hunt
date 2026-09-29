use leptos::prelude::*;
use leptos_meta::Title;

use super::{
    api::{Resolve, get_replies},
    model::{Class, Reply},
};
use crate::ui::{
    parts::{Avatar, Empty, PageHead, ago},
    toast::announce,
};

#[component]
pub fn RepliesPage() -> impl IntoView {
    let resolve = ServerAction::<Resolve>::new();
    announce(resolve, "Filed. The job moved to match.");
    let data = Resource::new(move || resolve.version().get(), |_| get_replies());
    view! {
        <Title text="Replies" />
        <PageHead title="Replies" sub="hunt reads replies to your applications and moves each job when it is sure. When it is not, it asks you here." />
        <Transition fallback=|| view! { <div class="box skeleton" style="height:240px"></div> }>
            {move || Suspend::new(async move {
                let replies = data.await.unwrap_or_default();
                if replies.is_empty() {
                    return view! {
                        <Empty title="No replies yet">
                            <p>"Replies show up within 15 minutes of arriving. hunt only reads mail about jobs you applied to."</p>
                            <a class="btn" href="/settings#gmail">"Connect Gmail"</a>
                        </Empty>
                    }.into_any();
                }
                let (asks, done): (Vec<_>, Vec<_>) = replies.into_iter().partition(|r| r.status == "needs_you");
                view! {
                    {(!asks.is_empty()).then(|| view! {
                        <section>
                            <h2>{format!("Needs you · {}", asks.len())}</h2>
                            <div class="stack">{asks.into_iter().map(|r| view! { <Ask r resolve /> }).collect_view()}</div>
                        </section>
                    })}
                    {(!done.is_empty()).then(|| view! {
                        <section>
                            <h2>"Handled"</h2>
                            <div class="box">{done.into_iter().map(|r| view! { <Handled r /> }).collect_view()}</div>
                        </section>
                    })}
                }.into_any()
            })}
        </Transition>
    }
}

fn job_link(r: &Reply) -> Option<impl IntoView + use<>> {
    match (r.job_id, r.company.clone(), r.title.clone()) {
        (Some(job), Some(company), Some(title)) => {
            Some(view! { <a class="chip outline" href=format!("/jobs/{job}")>{format!("{company} · {title}")}</a> })
        }
        _ => None,
    }
}

#[component]
fn Ask(r: Reply, resolve: ServerAction<Resolve>) -> impl IntoView {
    let id = r.id;
    let guess = r.class.clone().map(|c| {
        let sure = r.confidence.map(|c| format!(", {:.0}% sure", c * 100.0)).unwrap_or_default();
        format!("hunt thinks: {c}{sure}")
    });
    let choices = [
        (Class::Screen, "First call"),
        (Class::Interview, "Interview"),
        (Class::Offer, "Offer"),
        (Class::Rejection, "Rejection"),
        (Class::Other, "Not about this job"),
    ];
    view! {
        <article class="card reply-card">
            <header class="who">
                <Avatar name=r.sender.clone() />
                <div>
                    <b>{r.sender.clone()}</b>
                    <div class="co">{ago(r.at)}</div>
                </div>
                <div class="chips" style="margin-left:auto">{job_link(&r)}</div>
            </header>
            <p class="subject">{r.subject.clone()}</p>
            <blockquote>{r.snippet.clone()}</blockquote>
            <footer class="row">
                {guess.map(|g| view! { <span class="chip acc">{g}</span> })}
                <div class="segmented" role="group" aria-label="What is this reply?">
                    <span>"It is"</span>
                    {choices.into_iter().map(|(class, label)| view! {
                        <button disabled=move || resolve.pending().get() on:click=move |_| { resolve.dispatch(Resolve { id, class }); }>{label}</button>
                    }).collect_view()}
                </div>
            </footer>
        </article>
    }
}

#[component]
fn Handled(r: Reply) -> impl IntoView {
    view! {
        <div class="reply">
            <div>
                <div class="row" style="gap:8px"><b>{r.sender.clone()}</b><span class="co">{ago(r.at)}</span></div>
                <div class="subject">{r.subject.clone()}</div>
            </div>
            <div class="chips">
                {job_link(&r)}
                {r.class.clone().map(|c| view! { <span class="chip acc">{c}</span> })}
            </div>
        </div>
    }
}
