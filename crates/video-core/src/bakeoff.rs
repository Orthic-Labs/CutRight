//!
//! Style bake-offs and acceptance (CR-V2-B5-013).
//!
//! A bake-off fixes a single style direction as the **baseline** and
//! produces a small set of `Variant` records that differ from the baseline
//! only on the declared `axis_token` dimensions. The acceptance record
//! records the critical's `verdict` and the chosen `style_direction_id`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BakeOffError {
    #[error("bakeoff must contain at least 2 variants: id={0}")]
    TooFewVariants(String),
    #[error("variant must differ on at least one axis_token: id={0}")]
    NoDeclaredAxis(String),
    #[error("baseline style_direction_id missing: {0}")]
    MissingBaseline(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub style_direction_id: String,
    /// Declared axis tokens that differ from the baseline.
    pub axis_tokens: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeOff {
    pub id: String,
    pub version: String,
    pub baseline_style_direction_id: String,
    pub variants: Vec<Variant>,
    pub content_geometry_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeOffAcceptance {
    pub id: String,
    pub version: String,
    pub bakeoff_id: String,
    pub verdict: String,
    pub chosen_style_direction_id: String,
    pub rationale: String,
}

pub struct BakeOffService;

impl BakeOffService {
    pub fn validate(bakeoff: &BakeOff) -> Result<(), BakeOffError> {
        if bakeoff.baseline_style_direction_id.is_empty() {
            return Err(BakeOffError::MissingBaseline(bakeoff.id.clone()));
        }
        if bakeoff.variants.len() < 2 {
            return Err(BakeOffError::TooFewVariants(bakeoff.id.clone()));
        }
        for v in &bakeoff.variants {
            if v.axis_tokens.is_empty() {
                return Err(BakeOffError::NoDeclaredAxis(v.id.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bakeoff() -> BakeOff {
        BakeOff {
            id: "bo_1".to_string(),
            version: "v2".to_string(),
            baseline_style_direction_id: "sd_baseline".to_string(),
            variants: vec![
                Variant {
                    id: "v1".to_string(),
                    style_direction_id: "sd_a".to_string(),
                    axis_tokens: BTreeMap::from([(
                        "palette.warmth".to_string(),
                        "warm".to_string(),
                    )]),
                },
                Variant {
                    id: "v2".to_string(),
                    style_direction_id: "sd_b".to_string(),
                    axis_tokens: BTreeMap::from([(
                        "palette.warmth".to_string(),
                        "cool".to_string(),
                    )]),
                },
            ],
            content_geometry_id: "cg_1".to_string(),
        }
    }

    #[test]
    fn accepts_valid_bakeoff() {
        BakeOffService::validate(&sample_bakeoff()).expect("ok");
    }

    #[test]
    fn rejects_bakeoff_with_one_variant() {
        let mut b = sample_bakeoff();
        b.variants.pop();
        let err = BakeOffService::validate(&b).err().expect("err");
        assert!(matches!(err, BakeOffError::TooFewVariants(_)));
    }

    #[test]
    fn rejects_variant_without_axis_tokens() {
        let mut b = sample_bakeoff();
        b.variants[0].axis_tokens.clear();
        let err = BakeOffService::validate(&b).err().expect("err");
        assert!(matches!(err, BakeOffError::NoDeclaredAxis(_)));
    }
}
