use leptos::{html, prelude::*};
use leptos_router::{NavigateOptions, hooks::use_navigate};

use super::{
    api::SearchNow,
    icons::{Glyph, Icon},
};

#[derive(Clone, Copy, PartialEq)]
enum Command {
    Go(&'static str),
    Search,
}

const COMMANDS: [(&str, Glyph, Command, &str); 12] = [
    ("Today", Glyph::Today, Command::Go("/"), "G T"),
    ("Review applications", Glyph::Review, Command::Go("/review"), "G R"),
    ("Jobs worth a look", Glyph::Jobs, Command::Go("/jobs"), "G J"),
    ("Jobs to apply to yourself", Glyph::Jobs, Command::Go("/jobs?show=manual"), ""),
    ("Replies", Glyph::Replies, Command::Go("/replies"), "G E"),
    ("Pipeline", Glyph::Pipeline, Command::Go("/pipeline"), "G P"),
    ("Insights", Glyph::Insights, Command::Go("/insights"), "G I"),
    ("You and your profile", Glyph::You, Command::Go("/you"), "G Y"),
    ("Settings", Glyph::Settings, Command::Go("/settings"), "G S"),
    ("Log", Glyph::Log, Command::Go("/log"), "G L"),
    ("Search every source now", Glyph::Search, Command::Search, ""),
    ("Watch a company", Glyph::Settings, Command::Go("/settings#companies"), ""),
];

const SHEET: [(&str, &[(&str, &str)]); 3] = [
    (
        "Anywhere",
        &[
            ("⌘ K", "Open the command menu"),
            ("/", "Search jobs"),
            ("?", "Show these keys"),
            ("G then a letter", "Go to a page"),
        ],
    ),
    ("Review", &[("J  K", "Next and previous"), ("A", "Approve"), ("S", "Skip"), ("E", "Edit the letter")]),
    ("Jobs", &[("J  K", "Move"), ("Enter", "Open"), ("X", "Skip")]),
];

/// Global keys, the command menu and the key sheet.
#[component]
pub fn Keys() -> impl IntoView {
    let navigate = use_navigate();
    let (palette, sheet) = (RwSignal::new(false), RwSignal::new(false));
    let leader = RwSignal::new(false);
    let handle = window_event_listener(leptos::ev::keydown, move |event| {
        if (event.meta_key() || event.ctrl_key()) && event.key() == "k" {
            event.prevent_default();
            palette.update(|open| *open = !*open);
            return;
        }
        if event.key() == "Escape" {
            palette.set(false);
            sheet.set(false);
            return;
        }
        if event.meta_key() || event.ctrl_key() || typing(&event) || palette.get_untracked() {
            return;
        }
        if leader.get_untracked() {
            leader.set(false);
            let page = match event.key().as_str() {
                "t" => "/",
                "r" => "/review",
                "j" => "/jobs",
                "e" => "/replies",
                "p" => "/pipeline",
                "i" => "/insights",
                "y" => "/you",
                "l" => "/log",
                "s" => "/settings",
                _ => return,
            };
            navigate(page, NavigateOptions::default());
            return;
        }
        match event.key().as_str() {
            "g" => leader.set(true),
            "?" => sheet.update(|open| *open = !*open),
            "/" => {
                event.prevent_default();
                focus_search(&navigate);
            }
            _ => {}
        }
    });
    on_cleanup(move || handle.remove());

    view! {
        {move || palette.get().then(|| view! { <Palette open=palette /> })}
        {move || sheet.get().then(|| view! { <Sheet open=sheet /> })}
    }
}

fn focus_search(navigate: &impl Fn(&str, NavigateOptions)) {
    let search = document().query_selector("input[type=search]").ok().flatten();
    match search.and_then(|el| leptos::wasm_bindgen::JsCast::dyn_into::<leptos::web_sys::HtmlElement>(el).ok()) {
        Some(input) => drop(input.focus()),
        None => navigate("/jobs?focus=1", NavigateOptions::default()),
    }
}

#[component]
fn Palette(open: RwSignal<bool>) -> impl IntoView {
    let navigate = use_navigate();
    let search = expect_context::<ServerAction<SearchNow>>();
    let query = RwSignal::new(String::new());
    let active = RwSignal::new(0_usize);
    let input = NodeRef::<html::Input>::new();
    Effect::new(move || {
        if let Some(input) = input.get() {
            drop(input.focus());
        }
    });
    let matches = move || {
        let q = query.get().to_lowercase();
        COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, (label, ..))| label.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    };
    let run = Callback::new(move |index: usize| {
        open.set(false);
        match COMMANDS[index].2 {
            Command::Go(page) => navigate(page, NavigateOptions::default()),
            Command::Search => {
                search.dispatch(SearchNow {});
            }
        }
    });
    let keydown = move |event: leptos::ev::KeyboardEvent| {
        let found = matches();
        match event.key().as_str() {
            "ArrowDown" => {
                event.prevent_default();
                active.update(|a| *a = (*a + 1).min(found.len().saturating_sub(1)));
            }
            "ArrowUp" => {
                event.prevent_default();
                active.update(|a| *a = a.saturating_sub(1));
            }
            "Enter" => {
                if let Some(&index) = found.get(active.get_untracked()) {
                    run.run(index);
                }
            }
            _ => {}
        }
    };

    view! {
        <div class="scrim" on:click=move |_| open.set(false)>
            <div class="palette" role="dialog" aria-label="Command menu" on:click=|e| e.stop_propagation()>
                <div class="palette-input">
                    <Icon glyph=Glyph::Search />
                    <input node_ref=input type="text" placeholder="Go to a page or run an action" aria-label="Command"
                        prop:value=query on:input=move |e| { query.set(event_target_value(&e)); active.set(0); } on:keydown=keydown />
                    <span class="kbd">"esc"</span>
                </div>
                <ul class="palette-list" role="listbox">
                    {move || {
                        let found = matches();
                        if found.is_empty() {
                            return view! { <li class="none">"No command matches."</li> }.into_any();
                        }
                        found.into_iter().enumerate().map(|(n, index)| {
                            let (label, glyph, _, keys) = COMMANDS[index];
                            view! {
                                <li role="option" aria-selected=move || (active.get() == n).to_string()
                                    on:mouseenter=move |_| active.set(n) on:click=move |_| run.run(index)>
                                    <Icon glyph />
                                    <span>{label}</span>
                                    {(!keys.is_empty()).then(|| view! { <span class="kbd">{keys}</span> })}
                                </li>
                            }
                        }).collect_view().into_any()
                    }}
                </ul>
            </div>
        </div>
    }
}

#[component]
fn Sheet(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <div class="scrim" on:click=move |_| open.set(false)>
            <div class="sheet" role="dialog" aria-label="Keyboard shortcuts" on:click=|e| e.stop_propagation()>
                <div class="sheet-head"><h3>"Keyboard shortcuts"</h3><span class="kbd">"esc"</span></div>
                {SHEET.iter().map(|(area, keys)| view! {
                    <h4>{*area}</h4>
                    <dl>
                        {keys.iter().map(|(key, what)| view! {
                            <dt>{*what}</dt>
                            <dd>{key.split("  ").map(|k| view! { <span class="kbd">{k.to_string()}</span> }).collect_view()}</dd>
                        }).collect_view()}
                    </dl>
                }).collect_view()}
            </div>
        </div>
    }
}

pub fn typing(event: &leptos::ev::KeyboardEvent) -> bool {
    use leptos::wasm_bindgen::JsCast;
    event
        .target()
        .and_then(|t| t.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        .is_some_and(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT") || el.is_content_editable())
}
