//! Strong fixture: `saturation` is covered above and below the cap, so the
//! literal-zero mutant on line 6 is killed and the mutation layer must pass.

pub fn saturation(value: i64) -> i64 {
    if value > 100 {
        return 100;
    }
    value
}
