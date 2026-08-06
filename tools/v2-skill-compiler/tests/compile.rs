//! Integration tests for the v2 skill compiler, driven by committed
//! fixture trees so results are deterministic and offline.

use std::path::PathBuf;

use v2_skill_compiler::{compile, render_pack, render_topology};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn errors(report: &v2_skill_compiler::CompileReport) -> String {
    report.errors.join("\n")
}

#[test]
fn valid_tree_compiles_with_topological_order() {
    let report = compile(&fixture("valid"));
    assert!(report.ok(), "unexpected errors: {}", errors(&report));
    assert_eq!(report.skills.len(), 2);
    assert_eq!(report.topology_order, vec!["beta".to_string(), "alpha".to_string()]);
    let alpha = &report.skills[1];
    assert_eq!(alpha.id, "alpha");
    assert_eq!(alpha.order, 1);
    assert!(alpha.content_hash.starts_with("sha256:"));
    assert_eq!(alpha.dependencies, vec!["beta".to_string()]);
    assert_eq!(
        alpha.permissions,
        vec!["asset.request".to_string(), "render.sample".to_string()]
    );
    assert_eq!(alpha.resources.len(), 1);
    assert!(alpha.resources[0].sha256.starts_with("sha256:"));
    assert!(report.registry_present);
    assert_eq!(report.capability_count, 3);
    assert_eq!(report.schemas.len(), 1);
}

#[test]
fn pack_is_byte_identical_across_builds() {
    let first = compile(&fixture("valid"));
    let second = compile(&fixture("valid"));
    assert!(first.ok() && second.ok());
    assert_eq!(render_pack(&first), render_pack(&second));
    assert_eq!(render_topology(&first), render_topology(&second));
    // Canonical form must not embed absolute paths or timestamps.
    let pack = render_pack(&first);
    assert!(!pack.contains(env!("CARGO_MANIFEST_DIR")));
    assert!(pack.contains("\"pack_hash\":\"sha256:"));
}

#[test]
fn dangling_dependency_fixture_fails() {
    let report = compile(&fixture("dangling-dep"));
    assert!(!report.ok());
    let text = errors(&report);
    assert!(
        text.contains("[gamma] dangling dependency `ghost`"),
        "errors were:\n{text}"
    );
}

#[test]
fn external_path_fixture_fails() {
    let report = compile(&fixture("external-path"));
    assert!(!report.ok());
    let text = errors(&report);
    assert!(text.contains("external resource path rejected: ../outside.md"), "{text}");
    assert!(text.contains("absolute resource path rejected: /etc/passwd"), "{text}");
    assert!(
        text.contains("mutable/glob resource reference rejected: *.md"),
        "{text}"
    );
}

#[test]
fn undeclared_permission_fixture_fails() {
    let report = compile(&fixture("undeclared-permission"));
    assert!(!report.ok());
    let text = errors(&report);
    assert!(
        text.contains("[epsilon] undeclared permission `network.fetch`"),
        "{text}"
    );
}

#[test]
fn dependency_cycle_fixture_fails() {
    let report = compile(&fixture("cycle"));
    assert!(!report.ok());
    let text = errors(&report);
    assert!(text.contains("dependency cycle"), "{text}");
    assert!(text.contains("arc-a") && text.contains("arc-b"), "{text}");
}

#[test]
fn missing_skills_root_is_reported() {
    let report = compile(&fixture("no-registry").join("absent"));
    assert!(!report.ok());
    assert!(errors(&report).contains("missing skills root"));
}

#[test]
fn absent_registry_allows_permission_free_skills() {
    let report = compile(&fixture("no-registry"));
    assert!(report.ok(), "unexpected errors: {}", errors(&report));
    assert!(!report.registry_present);
    assert_eq!(report.skills.len(), 1);
    assert_eq!(report.skills[0].id, "zeta");
}
