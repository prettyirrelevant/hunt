use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    NavigateOptions,
    components::{A, Route, Router, Routes},
    hooks::{use_location, use_navigate},
    path,
};

use super::{api::get_nav, parts::ago};
use crate::{
    insights::views::{InsightsPage, LogPage, PipelinePage, TodayPage},
    jobs::views::{JobPage, JobsPage},
    profile::views::YouPage,
    review::views::ReviewPage,
    settings::views::SettingsPage,
    tracking::views::RepliesPage,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <script src="/vendor/echarts.min.js" defer></script>
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="hunt" href="/pkg/hunt.css" />
        <Title formatter=|page: String| format!("{page} · hunt") />
        <Router>
            <Shortcuts />
            <div class="app">
                <Sidebar />
                <main>
                    <Routes fallback=|| view! { <h1>"Nothing lives at this address"</h1> }>
                        <Route path=path!("/") view=TodayPage />
                        <Route path=path!("/review") view=ReviewPage />
                        <Route path=path!("/jobs") view=JobsPage />
                        <Route path=path!("/jobs/:id") view=JobPage />
                        <Route path=path!("/replies") view=RepliesPage />
                        <Route path=path!("/pipeline") view=PipelinePage />
                        <Route path=path!("/insights") view=InsightsPage />
                        <Route path=path!("/you") view=YouPage />
                        <Route path=path!("/log") view=LogPage />
                        <Route path=path!("/settings") view=SettingsPage />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let location = use_location();
    let data = Resource::new(move || location.pathname.get(), |_| get_nav());
    let link = move |href: &'static str, label: &'static str, count: fn(&super::api::Nav) -> i64, quiet: bool| {
        let current = move || {
            let path = location.pathname.get();
            if href == "/" { path == "/" } else { path.starts_with(href) }
        };
        view! {
            <A href=href attr:aria-current=move || current().then_some("page")>
                {label}
                <Transition>
                    {move || {
                        let n = data.get().and_then(Result::ok).map_or(0, |nav| count(&nav));
                        (n > 0).then(|| view! { <span class=if quiet { "count quiet num" } else { "count num" }>{n}</span> })
                    }}
                </Transition>
            </A>
        }
    };
    let none = |_: &super::api::Nav| 0;

    view! {
        <aside class="side">
            <A href="/" attr:class="brand"><b>"hunt"</b><span>"your job search"</span></A>
            <nav class="nav" aria-label="Pages">
                {link("/", "Today", none, false)}
                {link("/review", "Review", |n| n.ready, false)}
                {link("/jobs", "Jobs", |n| n.manual, true)}
                {link("/replies", "Replies", |n| n.replies, false)}
                {link("/pipeline", "Pipeline", none, false)}
                {link("/insights", "Insights", none, false)}
                <hr />
                {link("/you", "You", none, false)}
                {link("/log", "Log", none, false)}
                {link("/settings", "Settings", none, false)}
            </nav>
            <Transition>
                {move || data.get().and_then(Result::ok).map(|nav| {
                    let dot = if nav.ai_ready { "dot" } else { "dot warn" };
                    let when = |at: Option<_>| at.map_or_else(|| "never".into(), ago);
                    view! {
                        <div class="status" aria-label="System">
                            <h4>"System"</h4>
                            <div class="srow"><span>"AI"</span><span><i class=dot></i>{nav.ai}</span></div>
                            <div class="srow"><span>"Last search"</span><span class="num">{when(nav.last_sweep)}</span></div>
                            <div class="srow"><span>"Work queued"</span><span class="num">{nav.waiting}</span></div>
                            <div class="srow"><span>"Backup"</span><span class="num">{when(nav.last_backup)}</span></div>
                        </div>
                    }
                })}
            </Transition>
            <div class="keys"><span class="kbd">"g"</span>" then "<span class="kbd">"t"</span>" "<span class="kbd">"r"</span>" "<span class="kbd">"j"</span>" "<span class="kbd">"p"</span>" to jump"</div>
        </aside>
    }
}

/// `g` then a letter jumps to a page.
#[component]
fn Shortcuts() -> impl IntoView {
    let navigate = use_navigate();
    let leader = RwSignal::new(false);
    let handle = window_event_listener(leptos::ev::keydown, move |event| {
        if event.meta_key() || event.ctrl_key() || typing(&event) {
            return;
        }
        if leader.get_untracked() {
            leader.set(false);
            let page = match event.key().as_str() {
                "t" => "/",
                "r" => "/review",
                "j" => "/jobs",
                "p" => "/pipeline",
                "i" => "/insights",
                "y" => "/you",
                "l" => "/log",
                "s" => "/settings",
                _ => return,
            };
            navigate(page, NavigateOptions::default());
        } else if event.key() == "g" {
            leader.set(true);
        }
    });
    on_cleanup(move || handle.remove());
}

pub fn typing(event: &leptos::ev::KeyboardEvent) -> bool {
    use leptos::wasm_bindgen::JsCast;
    event
        .target()
        .and_then(|t| t.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT") || el.is_content_editable())
}
