use leptos::prelude::*;

#[derive(Clone, Copy)]
pub enum Glyph {
    Today,
    Review,
    Jobs,
    Replies,
    Pipeline,
    Insights,
    You,
    Settings,
    Log,
    Search,
    Arrow,
    Check,
    Upload,
    Up,
    Down,
}

#[component]
pub fn Icon(glyph: Glyph, #[prop(default = 16)] size: u32) -> impl IntoView {
    let shape = match glyph {
        Glyph::Today => view! { <path d="M3.5 10.5 12 3.5l8.5 7V19a1.5 1.5 0 0 1-1.5 1.5h-4v-6h-6v6H5A1.5 1.5 0 0 1 3.5 19z" /> }.into_any(),
        Glyph::Review => view! { <rect x="3.5" y="4.5" width="17" height="15" rx="2.5" /><path d="M3.5 13.5h4.5l1.5 2.5h5l1.5-2.5h4.5" /> }.into_any(),
        Glyph::Jobs => view! { <rect x="3.5" y="7.5" width="17" height="12" rx="2.5" /><path d="M8.5 7.5V6a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v1.5M3.5 13h17" /> }.into_any(),
        Glyph::Replies => view! { <rect x="3.5" y="5.5" width="17" height="13" rx="2.5" /><path d="m4 7.5 8 5.5 8-5.5" /> }.into_any(),
        Glyph::Pipeline => view! { <path d="M4 5.5h16l-6 7v5l-4 2v-7z" /> }.into_any(),
        Glyph::Insights => view! { <path d="M5.5 19.5v-6M12 19.5v-14M18.5 19.5v-9" /> }.into_any(),
        Glyph::You => view! { <circle cx="12" cy="8.5" r="3.75" /><path d="M4.5 20a7.5 7.5 0 0 1 15 0" /> }.into_any(),
        Glyph::Settings => view! { <path d="M4 7.5h9M17 7.5h3M4 16.5h3M11 16.5h9" /><circle cx="15" cy="7.5" r="2" /><circle cx="9" cy="16.5" r="2" /> }.into_any(),
        Glyph::Log => view! { <path d="M9 6.5h11M9 12h11M9 17.5h11M4.5 6.5h.01M4.5 12h.01M4.5 17.5h.01" /> }.into_any(),
        Glyph::Search => view! { <circle cx="11" cy="11" r="6.5" /><path d="m20 20-4.2-4.2" /> }.into_any(),
        Glyph::Arrow => view! { <path d="M5 12h14m-5.5-5.5L19 12l-5.5 5.5" /> }.into_any(),
        Glyph::Check => view! { <path d="m5 12.5 4.5 4.5L19 7.5" /> }.into_any(),
        Glyph::Upload => view! { <path d="M12 15.5V4.5m-4.5 4.5L12 4.5 16.5 9M4.5 19.5h15" /> }.into_any(),
        Glyph::Up => view! { <path d="m6.5 14.5 5.5-5.5 5.5 5.5" /> }.into_any(),
        Glyph::Down => view! { <path d="m6.5 9.5 5.5 5.5 5.5-5.5" /> }.into_any(),
    };
    view! {
        <svg class="icon" width=size height=size viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {shape}
        </svg>
    }
}

#[component]
pub fn Logo(#[prop(default = 26)] size: u32) -> impl IntoView {
    view! {
        <svg class="logo" width=size height=size viewBox="0 0 512 512" aria-hidden="true">
            <rect width="512" height="512" rx="116" fill="#2B45C4" />
            <g fill="none" stroke="#fff" stroke-width="64" stroke-linecap="round">
                <path d="M178 124V388" />
                <path d="M178 268a80 80 0 0 1 160 0v6" />
            </g>
            <circle cx="338" cy="378" r="36" fill="#fff" />
        </svg>
    }
}
