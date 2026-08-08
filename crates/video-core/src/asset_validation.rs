//!
//! Asset semantic validation, rights, and label locks (CR-V2-B5-015).
//!
//! The asset-validation lane produces an `ValidatedAssetReview` that records:
//! - `rights_record` — license, owner, expiry, territory
//! - `semantic_label` — short tag such as "LOGO", "FACE", "MUSIC", "AI"
//! - `lock_id` — a label lock that prevents the final render from
//!   rendering a sensitive asset without an explicit consent
//!
//! The creative critic reads this review and rejects any render-graph
//! that uses a locked asset without the matching `lock_id` consent.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetValidationError {
    #[error("rights record missing license: id={0}")]
    MissingLicense(String),
    #[error("expired rights: id={id}, expires_at={expires_at}")]
    Expired { id: String, expires_at: String },
    #[error("missing semantic label: id={0}")]
    MissingLabel(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightsRecord {
    pub id: String,
    pub license: String,
    pub owner: String,
    pub expires_at: String,
    pub territory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedAssetReview {
    pub id: String,
    pub version: String,
    pub asset_id: String,
    pub semantic_label: String,
    pub rights: RightsRecord,
    pub lock_id: Option<String>,
}

pub struct AssetValidationService;

impl AssetValidationService {
    pub fn validate(
        review: &ValidatedAssetReview,
        now_iso: &str,
    ) -> Result<(), AssetValidationError> {
        if review.rights.license.is_empty() {
            return Err(AssetValidationError::MissingLicense(review.id.clone()));
        }
        if review.semantic_label.is_empty() {
            return Err(AssetValidationError::MissingLabel(review.id.clone()));
        }
        if review.rights.expires_at.as_str() < now_iso {
            return Err(AssetValidationError::Expired {
                id: review.id.clone(),
                expires_at: review.rights.expires_at.clone(),
            });
        }
        Ok(())
    }

    pub fn is_locked(review: &ValidatedAssetReview) -> bool {
        review.lock_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ValidatedAssetReview {
        ValidatedAssetReview {
            id: "arv_1".to_string(),
            version: "v2".to_string(),
            asset_id: "asset_1".to_string(),
            semantic_label: "LOGO".to_string(),
            rights: RightsRecord {
                id: "rr_1".to_string(),
                license: "MIT".to_string(),
                owner: "Acme".to_string(),
                expires_at: "2099-01-01T00:00:00Z".to_string(),
                territory: "WW".to_string(),
            },
            lock_id: None,
        }
    }

    #[test]
    fn accepts_valid_review() {
        AssetValidationService::validate(&sample(), "2026-01-01T00:00:00Z").expect("ok");
    }

    #[test]
    fn rejects_missing_license() {
        let mut r = sample();
        r.rights.license = "".to_string();
        let err = AssetValidationService::validate(&r, "2026-01-01T00:00:00Z").expect_err("err");
        assert!(matches!(err, AssetValidationError::MissingLicense(_)));
    }

    #[test]
    fn rejects_expired() {
        let r = sample();
        let err = AssetValidationService::validate(&r, "2099-02-01T00:00:00Z").expect_err("err");
        assert!(matches!(err, AssetValidationError::Expired { .. }));
    }

    #[test]
    fn rejects_missing_label() {
        let mut r = sample();
        r.semantic_label = "".to_string();
        let err = AssetValidationService::validate(&r, "2026-01-01T00:00:00Z").expect_err("err");
        assert!(matches!(err, AssetValidationError::MissingLabel(_)));
    }
}
