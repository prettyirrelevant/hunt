use serde::{Deserialize, Serialize};

/// The AI's read of one job against your profile and your work.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Assessment {
    /// 0 to 100: how well the candidate matches what the job needs.
    pub fit: u8,
    pub verdict: Call,
    /// Two plain sentences, the way a friend who read the posting would put it.
    pub why: String,
    pub strengths: Vec<Strength>,
    pub gaps: Vec<Gap>,
    /// Warning signs in the posting itself, such as vague pay or unpaid tests.
    pub red_flags: Vec<String>,
    pub apply_via: ApplyVia,
    /// Only when the posting names an address to send applications to.
    pub apply_email: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Call {
    Apply,
    Maybe,
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ApplyVia {
    Email,
    Form,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Strength {
    pub requirement: String,
    /// The candidate's work that meets it.
    pub evidence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(schemars::JsonSchema))]
pub struct Gap {
    pub requirement: String,
    pub note: String,
}
