use hunt::insights::model::{Flow, Reach, flows};
use hunt::jobs::Stage;

fn reach(stage: Stage, shortlisted: bool, applied: bool, screened: bool, interviewed: bool) -> Reach {
    Reach { stage, shortlisted, applied, screened, interviewed }
}

fn flow(from: &str, to: &str, jobs: i64) -> Flow {
    Flow { from: from.into(), to: to.into(), jobs }
}

#[test]
fn each_job_flows_to_the_furthest_stage_it_reached() {
    let jobs = [
        reach(Stage::Filtered, false, false, false, false),
        reach(Stage::Filtered, false, false, false, false),
        reach(Stage::Skipped, true, false, false, false),
        reach(Stage::Rejected, true, true, true, false),
        reach(Stage::Offer, true, true, true, true),
    ];
    assert_eq!(
        flows(&jobs),
        vec![
            flow("Found", "Not a fit", 2),
            flow("Found", "Shortlisted", 3),
            flow("Shortlisted", "Skipped", 1),
            flow("Shortlisted", "Applied", 2),
            flow("Applied", "Screen", 2),
            flow("Screen", "Rejected", 1),
            flow("Screen", "Interview", 1),
            flow("Interview", "Offer", 1),
        ]
    );
}
