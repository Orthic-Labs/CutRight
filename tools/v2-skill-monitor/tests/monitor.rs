//! Integration tests for the v2 skill monitor, driven by committed fixtures.

use std::path::PathBuf;

use v2_skill_monitor::{
    monitor, render_report, MonitorReport, RootStatus, SkillState,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn status<'a>(report: &'a MonitorReport, id: &str) -> &'a v2_skill_monitor::SkillStatus {
    report
        .skills
        .iter()
        .find(|skill| skill.id == id)
        .unwrap_or_else(|| panic!("missing skill {id} in report"))
}

#[test]
fn mixed_tree_reports_typed_states() {
    let report = monitor(&fixture("mixed"));
    assert_eq!(report.root_status, RootStatus::Present);
    assert!(report.registry_present);

    let broken = status(&report, "broken-one");
    assert_eq!(broken.state, SkillState::Failed);
    assert!(broken.reasons.iter().any(|r| r == "dangling_dependency:ghost-skill"));
    assert!(broken.reasons.iter().any(|r| r == "external_path_resource:../escape.md"));
    assert!(broken.reasons.iter().any(|r| r == "undeclared_permission:network.fetch"));

    let degraded = status(&report, "degraded-one");
    assert_eq!(degraded.state, SkillState::Degraded);
    assert!(degraded.reasons.iter().any(|r| r == "missing_description"));
    assert!(degraded.reasons.iter().any(|r| r == "missing_notice"));

    assert_eq!(status(&report, "empty-dir").state, SkillState::Failed);
    assert!(status(&report, "empty-dir")
        .reasons
        .iter()
        .any(|r| r == "missing_skill_md"));

    assert_eq!(status(&report, "healthy-one").state, SkillState::Healthy);
    assert!(status(&report, "healthy-one").reasons.is_empty());

    assert_eq!(status(&report, "loop-a").state, SkillState::Failed);
    assert!(status(&report, "loop-a")
        .reasons
        .iter()
        .any(|r| r == "dependency_cycle"));
    assert_eq!(status(&report, "loop-b").state, SkillState::Failed);

    assert_eq!(report.summary(), (1, 1, 4));
    assert!(report.has_failures());
}

#[test]
fn missing_skills_root_is_reported_not_crashed() {
    let report = monitor(&fixture("empty-root"));
    assert_eq!(report.root_status, RootStatus::Missing);
    assert!(report.skills.is_empty());
    assert!(!report.has_failures());
}

#[test]
fn report_rendering_is_deterministic_and_sorted() {
    let first = render_report(&monitor(&fixture("mixed")));
    let second = render_report(&monitor(&fixture("mixed")));
    assert_eq!(first, second);
    let report = monitor(&fixture("mixed"));
    let ids: Vec<&str> = report
        .skills
        .iter()
        .map(|skill| skill.id.as_str())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
}
