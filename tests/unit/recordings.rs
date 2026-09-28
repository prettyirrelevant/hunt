use hunt::applying::service::video_expired;
use hunt::jobs::Stage;

#[test]
fn videos_of_active_applications_are_kept() {
    for stage in [Stage::Sending, Stage::Applied, Stage::Screen, Stage::Interview] {
        assert!(!video_expired(stage, 400), "{stage:?}");
    }
}

#[test]
fn videos_go_a_month_after_an_application_ends() {
    assert!(!video_expired(Stage::Rejected, 30));
    assert!(video_expired(Stage::Rejected, 31));
    assert!(video_expired(Stage::Ghosted, 45));
}

#[test]
fn offer_videos_are_kept_for_half_a_year() {
    assert!(!video_expired(Stage::Offer, 100));
    assert!(video_expired(Stage::Offer, 181));
}
