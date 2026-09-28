use std::path::PathBuf;

use hunt::profile::index::NeverRead;

fn home() -> PathBuf {
    directories::BaseDirs::new().unwrap().home_dir().to_path_buf()
}

fn never(patterns: &[&str]) -> NeverRead {
    NeverRead::new(&patterns.iter().map(ToString::to_string).collect::<Vec<_>>()).unwrap()
}

#[test]
fn a_folder_pattern_covers_everything_inside_it() {
    let never = never(&["client-work/"]);
    assert!(never.covers(&home().join("Developer/client-work/api/src/main.rs")));
    assert!(!never.covers(&home().join("Developer/hunt/src/main.rs")));
}

#[test]
fn a_file_pattern_matches_anywhere() {
    let never = never(&["*.secret"]);
    assert!(never.covers(&home().join("Developer/hunt/keys.secret")));
    assert!(!never.covers(&home().join("Developer/hunt/keys.toml")));
}

#[test]
fn a_home_pattern_is_anchored_to_home() {
    let never = never(&["~/Developer/acme"]);
    assert!(never.covers(&home().join("Developer/acme/app/lib.rs")));
    assert!(!never.covers(&home().join("Projects/Developer/acme/app/lib.rs")));
}

#[test]
fn a_path_outside_home_is_still_checked() {
    let never = never(&["client-work/"]);
    assert!(never.covers(&PathBuf::from("/Volumes/code/client-work/app.rs")));
}
