use linfa::prelude::*;
use linfa_logistic::LogisticRegression;
use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};

use crate::jobs::{Job, WorkMode};
use crate::profile::model::Profile;

/// Below this many labelled jobs, or this many of the rarer label, a model
/// would learn noise.
const MIN_EXAMPLES: usize = 30;
const MIN_PER_CLASS: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub examples: usize,
    /// Share of held-out jobs predicted correctly, when there were enough to hold out.
    pub accuracy: Option<f64>,
}

impl Model {
    pub fn train(rows: &[(Vec<f64>, bool)]) -> Option<Model> {
        let positives = rows.iter().filter(|(_, y)| *y).count();
        if rows.len() < MIN_EXAMPLES || positives.min(rows.len() - positives) < MIN_PER_CLASS {
            return None;
        }
        let fit = |rows: &[(Vec<f64>, bool)]| {
            let width = rows[0].0.len();
            let x =
                Array2::from_shape_vec((rows.len(), width), rows.iter().flat_map(|(x, _)| x.clone()).collect()).ok()?;
            let y = Array1::from_iter(rows.iter().map(|(_, y)| *y));
            let model = LogisticRegression::default().alpha(1.0).max_iterations(200).fit(&Dataset::new(x, y)).ok()?;
            // linfa picks its positive class from the data; orient the weights towards `true`.
            let sign = if model.labels().pos.class { 1.0 } else { -1.0 };
            Some((model.params().iter().map(|w| w * sign).collect::<Vec<f64>>(), model.intercept() * sign))
        };

        // Every fifth job is held out to measure accuracy.
        let (train, test): (Vec<_>, Vec<_>) = rows.iter().cloned().enumerate().partition(|(i, _)| i % 5 != 0);
        let train: Vec<_> = train.into_iter().map(|(_, row)| row).collect();
        let accuracy = fit(&train).map(|(weights, bias)| {
            let held_out = Model { weights, bias, examples: train.len(), accuracy: None };
            let hits = test.iter().filter(|(_, (x, y))| (held_out.predict(x) > 0.5) == *y).count();
            hits as f64 / test.len().max(1) as f64
        });

        let (weights, bias) = fit(rows)?;
        Some(Model { weights, bias, examples: rows.len(), accuracy })
    }

    pub fn predict(&self, features: &[f64]) -> f64 {
        let z = self.bias + self.weights.iter().zip(features).map(|(w, x)| w * x).sum::<f64>();
        1.0 / (1.0 + (-z).exp())
    }
}

/// The job's meaning plus the few facts that decide most calls.
pub fn features(job: &Job, embedding: &[f32], profile: &Profile, fit: Option<u8>) -> Vec<f64> {
    let skills: Vec<String> = profile.skills.iter().map(|s| s.to_lowercase()).collect();
    let overlap = if job.skills.is_empty() {
        0.0
    } else {
        job.skills.iter().filter(|s| skills.contains(&s.to_lowercase())).count() as f64 / job.skills.len() as f64
    };
    let facts = [
        f64::from(job.work_mode == WorkMode::Remote),
        f64::from(job.visa == Some(true) || job.relocation == Some(true)),
        f64::from(job.salary_min.is_some() || job.salary_max.is_some()),
        f64::from(job.seniority.as_ref().is_some_and(|s| profile.seniority.contains(s))),
        overlap,
        fit.map_or(0.5, |f| f64::from(f) / 100.0),
    ];
    embedding.iter().map(|&v| f64::from(v)).chain(facts).collect()
}

/// Cosine similarity to what you approved minus similarity to what you
/// skipped. Positive means the job looks more like your approvals.
pub fn rocchio(embedding: &[f32], liked: &[f32], skipped: &[f32]) -> f64 {
    cosine(embedding, liked) - cosine(embedding, skipped)
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
    f64::from(dot / (norm(a) * norm(b)).max(f32::EPSILON))
}
