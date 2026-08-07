//! Project-side trust façade.
//!
//! The project crate re-exports the trust types and provides the only
//! viewer-facing combine step that decides whether a final may be approved.

pub use video_security::trust::{
    compute_trust, forbid_model_override, HashPair, SignedItem, TrustComputation,
    TrustFailureRecord, TrustInputs, TrustOverall, TrustStatus,
};

/// Project-side policy: a finalize/package action is allowed only when
/// `can_finalize` is true.
pub fn may_finalize(c: &TrustComputation) -> bool {
    c.can_finalize
}

/// Confirm the authored disposition equals the floor; the model layer is
/// not permitted to authoritatively determine trust.
pub fn confirm_authoritative(
    proposed: TrustOverall,
    c: &TrustComputation,
) -> Result<TrustOverall, video_security::trust::TrustError> {
    forbid_model_override(proposed, c.status.overall.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn may_finalize_only_when_pass() {
        let c = compute_trust(&TrustInputs::default());
        assert!(may_finalize(&c));
    }
}
