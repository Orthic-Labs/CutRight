// Narrative arc templates (Book 4 lane C, B4-017).
//
// Approved arc library for long-form, shorts, explainers, ads, and
// stories, expressed as constrained templates with required roles.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcKind {
    LongForm,
    Shorts,
    Explainer,
    Ad,
    Story,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleSpec {
    pub role: String,
    pub min_count: u32,
    pub max_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArcTemplate {
    pub id: String,
    pub kind: ArcKind,
    pub roles: Vec<RoleSpec>,
}

pub fn library() -> Vec<ArcTemplate> {
    vec![
        ArcTemplate {
            id: "long-form.story".into(),
            kind: ArcKind::LongForm,
            roles: vec![
                RoleSpec { role: "hook".into(), min_count: 1, max_count: 1 },
                RoleSpec { role: "setup".into(), min_count: 1, max_count: 3 },
                RoleSpec { role: "payoff".into(), min_count: 1, max_count: 1 },
                RoleSpec { role: "cta".into(), min_count: 0, max_count: 1 },
            ],
        },
        ArcTemplate {
            id: "shorts.hook-payoff".into(),
            kind: ArcKind::Shorts,
            roles: vec![
                RoleSpec { role: "hook".into(), min_count: 1, max_count: 1 },
                RoleSpec { role: "payoff".into(), min_count: 1, max_count: 1 },
            ],
        },
        ArcTemplate {
            id: "explainer.context-claim-evidence".into(),
            kind: ArcKind::Explainer,
            roles: vec![
                RoleSpec { role: "context".into(), min_count: 1, max_count: 1 },
                RoleSpec { role: "claim".into(), min_count: 1, max_count: 1 },
                RoleSpec { role: "evidence".into(), min_count: 1, max_count: 3 },
            ],
        },
    ]
}

/// Validate that a list of `(role, count)` pairs satisfies the arc.
pub fn validate_arc(template: &ArcTemplate, role_counts: &[(&str, u32)]) -> bool {
    for spec in &template.roles {
        let count = role_counts
            .iter()
            .find(|(r, _)| *r == spec.role)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        if count < spec.min_count || count > spec.max_count {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_has_long_form_short_explainer() {
        let l = library();
        assert!(l.iter().any(|a| matches!(a.kind, ArcKind::LongForm)));
        assert!(l.iter().any(|a| matches!(a.kind, ArcKind::Shorts)));
        assert!(l.iter().any(|a| matches!(a.kind, ArcKind::Explainer)));
    }

    #[test]
    fn long_form_arc_validates_with_required_roles() {
        let l = library();
        let t = l.iter().find(|a| matches!(a.kind, ArcKind::LongForm)).unwrap();
        let counts = vec![("hook", 1), ("setup", 1), ("payoff", 1)];
        assert!(validate_arc(t, &counts));
    }

    #[test]
    fn arc_invalid_when_role_missing() {
        let l = library();
        let t = l.iter().find(|a| matches!(a.kind, ArcKind::LongForm)).unwrap();
        let counts = vec![("hook", 1), ("setup", 1)]; // missing payoff
        assert!(!validate_arc(t, &counts));
    }
}