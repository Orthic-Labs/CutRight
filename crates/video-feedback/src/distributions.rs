//! Per-axis weight distributions over decision reasons.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Probability-like weights keyed by decision reason. Sum is informational;
/// do not assume it is normalised.
pub type AxisDistribution = BTreeMap<String, f64>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistributionSample {
    pub distribution: AxisDistribution,
    pub sample_count: u32,
    pub variance: f64,
}
