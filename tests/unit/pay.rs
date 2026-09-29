use hunt::jobs::pay;

#[test]
fn known_currencies_use_their_symbol() {
    assert_eq!(pay(Some(150_000), Some(250_000), Some("USD")).as_deref(), Some("$150k–250k"));
    assert_eq!(pay(Some(80_000), None, Some("eur")).as_deref(), Some("€80k"));
    assert_eq!(pay(None, Some(120_000), Some("GBP")).as_deref(), Some("£120k"));
}

#[test]
fn other_currencies_follow_the_amount() {
    assert_eq!(pay(Some(878_000), Some(1_054_000), Some("SEK")).as_deref(), Some("878k–1.1M SEK"));
}

#[test]
fn a_range_that_rounds_to_one_amount_shows_once() {
    assert_eq!(pay(Some(149_800), Some(150_200), Some("USD")).as_deref(), Some("$150k"));
}

#[test]
fn no_amount_shows_nothing() {
    assert_eq!(pay(None, None, Some("USD")), None);
    assert_eq!(pay(Some(90_000), None, None).as_deref(), Some("90k"));
}
