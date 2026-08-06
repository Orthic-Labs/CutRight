//! Weak fixture: `clamp` has no boundary test (value == max), so the
//! `>` → `>=` mutant on line 3 survives. The gauntlet must report Failed.

pub fn clamp(value: i64, max: i64) -> i64 {
    if value > max {
        return max;
    }
    value
}
