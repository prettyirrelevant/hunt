use std::time::Duration;

use leptos::{prelude::*, server_fn::ServerFn};

use super::{
    error::Error,
    icons::{Glyph, Icon},
};

#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Done,
    Failed,
}

#[derive(Clone)]
struct Toast {
    id: u64,
    text: String,
    tone: Tone,
}

#[derive(Clone, Copy)]
pub struct Toasts {
    list: RwSignal<Vec<Toast>>,
    next: RwSignal<u64>,
}

impl Toasts {
    pub fn new() -> Toasts {
        Toasts { list: RwSignal::new(vec![]), next: RwSignal::new(0) }
    }

    pub fn show(self, text: impl Into<String>, tone: Tone) {
        let id = self.next.get_untracked();
        self.next.set(id + 1);
        self.list.update(|list| {
            list.push(Toast { id, text: text.into(), tone });
            if list.len() > 3 {
                list.remove(0);
            }
        });
        let stay = if tone == Tone::Failed { 7 } else { 3 };
        set_timeout(move || self.dismiss(id), Duration::from_secs(stay));
    }

    fn dismiss(self, id: u64) {
        self.list.update(|list| list.retain(|t| t.id != id));
    }
}

impl Default for Toasts {
    fn default() -> Toasts {
        Toasts::new()
    }
}

/// Shows `done` when `action` succeeds, and its error when it fails.
pub fn announce<S>(action: ServerAction<S>, done: &'static str)
where
    S: ServerFn<Error = Error> + Clone + Send + Sync + 'static,
    S::Output: Clone + Send + Sync + 'static,
{
    let toasts = expect_context::<Toasts>();
    Effect::new(move || match action.value().get() {
        Some(Ok(_)) => toasts.show(done, Tone::Done),
        Some(Err(err)) => toasts.show(err.to_string(), Tone::Failed),
        None => {}
    });
}

#[component]
pub fn Toaster() -> impl IntoView {
    let toasts = expect_context::<Toasts>();
    view! {
        <div class="toaster" role="status" aria-live="polite">
            <For each=move || toasts.list.get() key=|t| t.id let:toast>
                <div class="toast" class:failed=toast.tone == Tone::Failed>
                    {match toast.tone {
                        Tone::Done => view! { <span class="mark"><Icon glyph=Glyph::Check size=14 /></span> }.into_any(),
                        Tone::Failed => view! { <span class="mark">"!"</span> }.into_any(),
                    }}
                    <span class="text">{toast.text}</span>
                    <button class="close" aria-label="Dismiss" on:click=move |_| toasts.dismiss(toast.id)>"×"</button>
                </div>
            </For>
        </div>
    }
}
