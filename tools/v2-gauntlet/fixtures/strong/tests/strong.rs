use strong_sample::saturation;

#[test]
fn saturation_below_cap_unchanged() {
    assert_eq!(saturation(50), 50);
}

#[test]
fn saturation_above_cap_capped() {
    assert_eq!(saturation(200), 100);
}

#[test]
fn saturation_at_cap_unchanged() {
    assert_eq!(saturation(100), 100);
}

#[test]
fn saturation_just_above_cap() {
    assert_eq!(saturation(101), 100);
}
