use weak_sample::clamp;

#[test]
fn clamp_below_max_unchanged() {
    assert_eq!(clamp(5, 10), 5);
}

#[test]
fn clamp_above_max_capped() {
    assert_eq!(clamp(15, 10), 10);
}

// Deliberate weakness: no test covers the boundary value == max, so the
// ge-flip mutant on the comparison line survives.
