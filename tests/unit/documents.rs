use hunt::config::Contact;
use hunt::profile::index::read_cv;
use hunt::review::{
    documents,
    model::{Cv, Role, SkillGroup},
};

fn contact() -> Contact {
    Contact {
        name: "Ada Test".into(),
        email: "ada@example.com".into(),
        links: vec!["https://github.com/ada".into()],
        ..Default::default()
    }
}

fn cv() -> Cv {
    Cv {
        headline: "Backend engineer".into(),
        summary: "Builds payment systems in Rust.".into(),
        skills: vec![SkillGroup { label: "Languages".into(), items: vec!["Rust".into(), "Go".into()] }],
        experience: vec![Role {
            company: "Acme".into(),
            title: "Engineer".into(),
            dates: "2021 – 2024".into(),
            location: "Remote".into(),
            bullets: vec!["Cut p99 latency from 180 ms to 40 ms.".into()],
        }],
        ..Default::default()
    }
}

#[test]
fn a_cv_renders_to_pdf() {
    let pdf = documents::cv_pdf(&contact(), &cv()).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn an_uploaded_pdf_cv_is_read_as_text() {
    let pdf = documents::cv_pdf(&contact(), &cv()).unwrap();
    let text = read_cv("cv.pdf", &pdf).unwrap();
    assert!(text.contains("Ada Test"), "{text}");
    assert!(text.contains("Cut p99 latency from 180 ms to 40 ms."), "{text}");
}

#[test]
fn a_cover_letter_renders_to_pdf() {
    let pdf = documents::letter_pdf(&contact(), "Acme", "Dear team,\n\nI would like to join.").unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}
