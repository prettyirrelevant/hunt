use hunt::matching::learn::{Model, rocchio};

fn rows(n: usize) -> Vec<(Vec<f64>, bool)> {
    (0..n)
        .map(|i| {
            let remote = i % 2 == 0;
            (vec![f64::from(remote), (i % 7) as f64 / 7.0], remote)
        })
        .collect()
}

#[test]
fn a_clear_preference_is_learned() {
    let model = Model::train(&rows(60)).expect("enough data");
    assert!(model.predict(&[1.0, 0.3]) > 0.8);
    assert!(model.predict(&[0.0, 0.3]) < 0.2);
    assert!(model.accuracy.unwrap() > 0.9);
}

#[test]
fn too_few_decisions_train_nothing() {
    assert!(Model::train(&rows(12)).is_none());
}

#[test]
fn one_sided_decisions_train_nothing() {
    let all_yes: Vec<_> = (0..40).map(|i| (vec![f64::from(i)], true)).collect();
    assert!(Model::train(&all_yes).is_none());
}

#[test]
fn rocchio_points_towards_what_you_picked() {
    let liked = [1.0, 0.0];
    let skipped = [0.0, 1.0];
    assert!(rocchio(&[0.9, 0.1], &liked, &skipped) > 0.0);
    assert!(rocchio(&[0.1, 0.9], &liked, &skipped) < 0.0);
}
