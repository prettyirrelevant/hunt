use std::sync::LazyLock;

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use typst::foundations::{Dict, IntoValue};
use typst_as_lib::{TypstEngine, TypstTemplateMainFile, typst_kit_options::TypstKitFontOptions};
use typst_layout::PagedDocument;

use super::model::Cv;
use crate::config::Contact;

type Engine = TypstEngine<TypstTemplateMainFile>;

static CV: LazyLock<Engine> = LazyLock::new(|| engine(include_str!("templates/cv.typ")));
static LETTER: LazyLock<Engine> = LazyLock::new(|| engine(include_str!("templates/letter.typ")));

fn engine(template: &'static str) -> Engine {
    TypstEngine::builder()
        .main_file(template)
        .search_fonts_with(TypstKitFontOptions::default().include_system_fonts(false))
        .build()
}

pub fn cv_pdf(contact: &Contact, cv: &Cv) -> Result<Vec<u8>> {
    render(&CV, &json!({ "contact": contact, "cv": cv }))
}

pub fn letter_pdf(contact: &Contact, company: &str, letter: &str) -> Result<Vec<u8>> {
    let date = chrono::Local::now().format("%-d %B %Y").to_string();
    render(&LETTER, &json!({ "contact": contact, "company": company, "letter": letter, "date": date }))
}

fn render(engine: &Engine, data: &Value) -> Result<Vec<u8>> {
    let mut inputs = Dict::new();
    inputs.insert("data".into(), data.to_string().into_value());
    let document: PagedDocument = engine.compile_with_input(inputs).output.map_err(|e| anyhow!("{e:?}"))?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|e| anyhow!("{e:?}"))
}
