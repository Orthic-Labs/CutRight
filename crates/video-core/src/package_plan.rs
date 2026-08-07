//!
//! Thumbnails, title-cards, brand-kits, and package asset plans (CR-V2-B5-016).
//!
//! The package output is a `PackagePlan` that binds together:
//! - one or more thumbnails
//! - one or more title-cards
//! - zero or one brand-kit (the visual identity card for the package)
//! - a list of package assets
//!
//! Each thumbnail and title-card references a `style_direction_id` and an
//! `evidence_ref`. The plan is rejected if any required binding is missing.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageAssetError {
    #[error("package requires at least one thumbnail: id={0}")]
    NoThumbnail(String),
    #[error("package requires at least one title-card: id={0}")]
    NoTitleCard(String),
    #[error("thumbnail requires style_direction_id and evidence_ref: id={0}")]
    UnboundThumbnail(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub id: String,
    pub style_direction_id: String,
    pub evidence_ref: String,
    pub aspect_ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleCard {
    pub id: String,
    pub text: String,
    pub style_direction_id: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandKitRef {
    pub id: String,
    pub brand_card_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAsset {
    pub id: String,
    pub kind: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePlan {
    pub id: String,
    pub version: String,
    pub creative_plan_id: String,
    pub thumbnails: Vec<Thumbnail>,
    pub title_cards: Vec<TitleCard>,
    pub brand_kit: Option<BrandKitRef>,
    pub assets: Vec<PackageAsset>,
}

pub struct PackagePlanService;

impl PackagePlanService {
    pub fn validate(plan: &PackagePlan) -> Result<(), PackageAssetError> {
        if plan.thumbnails.is_empty() {
            return Err(PackageAssetError::NoThumbnail(plan.id.clone()));
        }
        if plan.title_cards.is_empty() {
            return Err(PackageAssetError::NoTitleCard(plan.id.clone()));
        }
        for t in &plan.thumbnails {
            if t.style_direction_id.is_empty() || t.evidence_ref.is_empty() {
                return Err(PackageAssetError::UnboundThumbnail(t.id.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PackagePlan {
        PackagePlan {
            id: "pp_1".to_string(),
            version: "v2".to_string(),
            creative_plan_id: "cp_1".to_string(),
            thumbnails: vec![Thumbnail {
                id: "thumb_1".to_string(),
                style_direction_id: "sd_1".to_string(),
                evidence_ref: "evidence:ev_1".to_string(),
                aspect_ratio: "16:9".to_string(),
            }],
            title_cards: vec![TitleCard {
                id: "tc_1".to_string(),
                text: "Title".to_string(),
                style_direction_id: "sd_1".to_string(),
                evidence_ref: "evidence:ev_1".to_string(),
            }],
            brand_kit: Some(BrandKitRef {
                id: "bk_1".to_string(),
                brand_card_id: "bc_1".to_string(),
            }),
            assets: vec![],
        }
    }

    #[test]
    fn accepts_valid_package() {
        PackagePlanService::validate(&sample()).expect("ok");
    }

    #[test]
    fn rejects_no_thumbnail() {
        let mut p = sample();
        p.thumbnails.clear();
        let err = PackagePlanService::validate(&p).err().expect("err");
        assert!(matches!(err, PackageAssetError::NoThumbnail(_)));
    }

    #[test]
    fn rejects_no_title_card() {
        let mut p = sample();
        p.title_cards.clear();
        let err = PackagePlanService::validate(&p).err().expect("err");
        assert!(matches!(err, PackageAssetError::NoTitleCard(_)));
    }

    #[test]
    fn rejects_unbound_thumbnail() {
        let mut p = sample();
        p.thumbnails[0].style_direction_id = "".to_string();
        let err = PackagePlanService::validate(&p).err().expect("err");
        assert!(matches!(err, PackageAssetError::UnboundThumbnail(_)));
    }
}
