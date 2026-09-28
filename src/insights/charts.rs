use charming::{
    Chart, WasmRenderer,
    component::{Axis, Grid, Legend},
    element::{AxisType, ItemStyle, Label, LineStyle, TextStyle, Tooltip, Trigger},
    series::{Bar, Sankey, SankeyNode},
};

use super::model::{Flow, Week};

fn token(name: &str) -> String {
    web_sys::window()
        .and_then(|w| w.document()?.document_element().map(|root| (w, root)))
        .and_then(|(w, root)| w.get_computed_style(&root).ok().flatten())
        .and_then(|style| style.get_property_value(name).ok())
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

/// Waits a frame so the chart's element is in the page.
pub fn draw(id: &'static str, chart: Chart) {
    leptos::prelude::request_animation_frame(move || {
        if let Err(err) = WasmRenderer::new_opt(None, None).render(id, &chart) {
            leptos::logging::warn!("chart {id} failed: {err:?}");
        }
    });
}

pub fn sankey(flows: &[Flow]) -> Chart {
    let colour = |name: &str| match name {
        "Offer" | "Interview" | "Screen" => token("--ok"),
        "Rejected" => token("--crit"),
        "No reply" => token("--warn"),
        "Not a fit" | "Hidden" | "Skipped" | "Closed" | "Withdrawn" => token("--muted"),
        "Waiting" | "Waiting to score" | "In review" => token("--line"),
        _ => token("--accent"),
    };
    let mut names: Vec<&str> = vec![];
    for flow in flows {
        for name in [flow.from.as_str(), flow.to.as_str()] {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    let nodes: Vec<SankeyNode> =
        names.iter().map(|&n| SankeyNode::new(n).item_style(ItemStyle::new().color(colour(n)))).collect();
    let links: Vec<(String, String, f64)> =
        flows.iter().map(|f| (f.from.clone(), f.to.clone(), f.jobs as f64)).collect();
    Chart::new().tooltip(Tooltip::new().trigger(Trigger::Item)).series(
        Sankey::new()
            .left(16)
            .right(120)
            .top(20)
            .bottom(20)
            .nodes(nodes)
            .links(links)
            .label(Label::new().color(token("--ink")).formatter("{b}  {c}"))
            .line_style(LineStyle::new().color("gradient").opacity(0.3).curveness(0.5)),
    )
}

pub fn weeks(weeks: &[Week]) -> Chart {
    let text = TextStyle::new().color(token("--muted"));
    Chart::new()
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .legend(Legend::new().bottom(0).text_style(text.clone()))
        .grid(Grid::new().left(36).right(12).top(16).bottom(48))
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(weeks.iter().map(|w| w.week.clone()).collect())
                .axis_label(charming::element::AxisLabel::new().color(token("--muted"))),
        )
        .y_axis(
            Axis::new().type_(AxisType::Value).axis_label(charming::element::AxisLabel::new().color(token("--muted"))),
        )
        .series(
            Bar::new()
                .name("Applied")
                .item_style(ItemStyle::new().color(token("--accent")))
                .data(weeks.iter().map(|w| w.applied as f64).collect()),
        )
        .series(
            Bar::new()
                .name("Heard back")
                .item_style(ItemStyle::new().color(token("--ok")))
                .data(weeks.iter().map(|w| w.heard_back as f64).collect()),
        )
}
