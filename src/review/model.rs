use serde::{Deserialize, Serialize};

/// A CV tailored to one job. Contact details are not part of it: they are
/// added locally when the PDF is made, so the AI never sees them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Cv {
    pub headline: String,
    pub summary: String,
    pub skills: Vec<SkillGroup>,
    pub experience: Vec<Role>,
    pub projects: Vec<Project>,
    pub education: Vec<Education>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct SkillGroup {
    pub label: String,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Role {
    pub company: String,
    pub title: String,
    pub dates: String,
    pub location: String,
    pub bullets: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Project {
    pub name: String,
    pub stack: Vec<String>,
    pub bullets: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Education {
    pub school: String,
    pub degree: String,
    pub dates: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Change {
    pub kind: ChangeKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Add,
    Move,
    Cut,
}

/// Everything the AI writes for one application.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Tailored {
    pub cv: Cv,
    pub changes: Vec<Change>,
    /// The cover letter body, without greeting details the AI cannot know.
    pub letter: String,
    pub email_subject: String,
    /// Short answers to questions application forms often ask.
    pub answers: Vec<Answer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Answer {
    pub question: String,
    pub answer: String,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Draft {
    pub job_id: i64,
    pub cv: sqlx::types::Json<Cv>,
    pub cv_changes: sqlx::types::Json<Vec<Change>>,
    pub letter: String,
    pub email_to: Option<String>,
    pub email_subject: Option<String>,
    pub answers: sqlx::types::Json<Vec<Answer>>,
    pub provider: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
