use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    components::{A, Route, Router, Routes},
    hooks::use_location,
    path,
};

use super::{
    api::{Nav, SearchNow, get_nav},
    icons::{Glyph, Icon, Logo},
    keys::Keys,
    parts::ago,
    toast::{Toaster, Toasts},
};
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
                <link rel="icon" type="image/svg+xml" href="/logo.svg" />
                <meta name="theme-color" content="#2B45C4" />
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
    provide_context(ServerAction::<SearchNow>::new());
    provide_context(Toasts::new());
    view! {
        <Stylesheet id="hunt" href="/pkg/hunt.css" />
        <Title formatter=|page: String| format!("{page} · hunt") />
        <Router>
            <Keys />
            <Toaster />
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
    let search = expect_context::<ServerAction<SearchNow>>();
    let tick = RwSignal::new(0_u32);
    #[cfg(feature = "hydrate")]
    {
        let handle = set_interval_with_handle(move || tick.update(|t| *t += 1), std::time::Duration::from_secs(15));
        on_cleanup(move || {
            if let Ok(handle) = handle {
                handle.clear();
            }
        });
    }
    let data = Resource::new(move || (location.pathname.get(), tick.get(), search.version().get()), |_| get_nav());
    let nav = move || data.get().and_then(Result::ok);
    let link = move |href: &'static str, label: &'static str, glyph: Glyph, count: fn(&Nav) -> i64| {
        let current = move || {
            let path = location.pathname.get();
            if href == "/" { path == "/" } else { path.starts_with(href) }
        };
        view! {
            <A href=href attr:aria-current=move || current().then_some("page")>
                <Icon glyph />
                <span class="label">{label}</span>
                <Transition>
                    {move || {
                        let n = nav().map_or(0, |nav| count(&nav));
                        (n > 0).then(|| view! { <span class="count num">{n}</span> })
                    }}
                </Transition>
            </A>
        }
    };
    let none = |_: &Nav| 0;

    view! {
        <aside class="side">
            <A href="/" attr:class="brand"><Logo size=24 /><b>"hunt"</b></A>
            <nav class="nav" aria-label="Pages">
                {link("/", "Today", Glyph::Today, none)}
                {link("/review", "Review", Glyph::Review, |n| n.ready)}
                {link("/jobs", "Jobs", Glyph::Jobs, |n| n.manual)}
                {link("/replies", "Replies", Glyph::Replies, |n| n.replies)}
                <span class="group">"Progress"</span>
                {link("/pipeline", "Pipeline", Glyph::Pipeline, none)}
                {link("/insights", "Insights", Glyph::Insights, none)}
                <span class="group">"Setup"</span>
                {link("/you", "You", Glyph::You, none)}
                {link("/settings", "Settings", Glyph::Settings, none)}
                {link("/log", "Log", Glyph::Log, none)}
            </nav>
            <Transition>
                {move || nav().map(|nav| {
                    let searching = nav.searching || search.pending().get();
                    let state = if searching {
                        "Searching every source".to_string()
                    } else {
                        nav.last_sweep.map_or_else(|| "Has not searched yet".into(), |at| format!("Searched {}", ago(at)))
                    };
                    let (dot, ai) = if nav.ai_ready { ("dot", nav.ai.clone()) } else { ("dot warn", nav.ai.clone()) };
                    view! {
                        <div class="status" aria-live="polite">
                            <div class="state"><i class="pulse" class:on=searching></i><span>{state}</span></div>
                            <div class="srow"><span>"AI"</span><span><i class=dot></i>{ai}</span></div>
                            {(nav.waiting > 0).then(|| view! { <div class="srow"><span>"Queued"</span><span class="num">{nav.waiting}</span></div> })}
                            <button class="btn small ghost wide" disabled=searching on:click=move |_| { search.dispatch(SearchNow {}); }>
                                <Icon glyph=Glyph::Search size=14 />{if searching { "Searching…" } else { "Search now" }}
                            </button>
                        </div>
                    }
                })}
            </Transition>
        </aside>
    }
}
