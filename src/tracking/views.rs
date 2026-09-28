use leptos::prelude::*;
use leptos_meta::Title;

use super::{
    api::{Resolve, get_replies},
    model::{Class, Reply},
};
use crate::ui::parts::{Empty, ago};

#[component]
pub fn RepliesPage() -> impl IntoView {
    let resolve = ServerAction::<Resolve>::new();
    let data = Resource::new(move || resolve.version().get(), |_| get_replies());
    view! {
        <Title text="Replies" />
        <h1>"Replies"</h1>
        <p class="sub">"hunt reads your inbox for replies to your applications and moves each job when it is sure. When it is not, it asks you here. Mail about anything else is left alone."</p>
        <Transition fallback=|| view! { <p class="sub">"Loading…"</p> }>
            {move || Suspend::new(async move {
                let replies = data.await.unwrap_or_default();
                if replies.is_empty() {
                    return view! { <Empty title="No replies yet"><p>"Connect Gmail in Settings. Replies show up here within 15 minutes of arriving."</p></Empty> }.into_any();
                }
                let (asks, done): (Vec<_>, Vec<_>) = replies.into_iter().partition(|r| r.status == "needs_you");
                view! {
                    {(!asks.is_empty()).then(|| view! {
                        <section>
                            <h2>"Needs you"</h2>
                            <div class="box">{asks.into_iter().map(|r| view! { <Row r resolve ask=true /> }).collect_view()}</div>
                        </section>
                    })}
                    <section>
                        <h2>"Handled"</h2>
                        <div class="box">{done.into_iter().map(|r| view! { <Row r resolve ask=false /> }).collect_view()}</div>
                    </section>
                }.into_any()
            })}
        </Transition>
    }
}

#[component]
fn Row(r: Reply, resolve: ServerAction<Resolve>, ask: bool) -> impl IntoView {
    let id = r.id;
    let pick = move |class: Class| {
        move |_| {
            resolve.dispatch(Resolve { id, class });
        }
    };
    let job = match (r.job_id, r.company, r.title) {
        (Some(job), Some(company), Some(title)) => {
            Some(view! { <a href=format!("/jobs/{job}")>{format!("{company} · {title}")}</a> })
        }
        _ => None,
    };
    let sure = r.confidence.map(|c| format!("{:.0}% sure", c * 100.0)).unwrap_or_default();
    view! {
        <div class="reply">
            <div>
                <b>{r.sender}</b>" · "<span class="muted">{ago(r.at)}</span>
                <div>{r.subject}</div>
                <div class="ev">{r.snippet}</div>
                <div class="row" style="margin-top:6px">{job}{r.class.map(|c| view! { <span class="chip acc">{c}</span> })}<span class="chip">{sure}</span></div>
            </div>
            {ask.then(|| view! {
                <div class="row">
                    <button class="btn small primary" on:click=pick(Class::Screen)>"Screen"</button>
                    <button class="btn small" on:click=pick(Class::Interview)>"Interview"</button>
                    <button class="btn small" on:click=pick(Class::Offer)>"Offer"</button>
                    <button class="btn small danger" on:click=pick(Class::Rejection)>"Rejection"</button>
                    <button class="btn small" on:click=pick(Class::Other)>"Not about this job"</button>
                </div>
            })}
        </div>
    }
}
