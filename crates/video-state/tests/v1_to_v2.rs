use std::fs;
use std::path::PathBuf;

use video_state::migrate::{MigrationRunner, MigrationStep};
use video_state::migrations::v2::{
    v1_to_v2_plan, FrozenStep, FROM_VERSION, FROZEN_STEPS, TO_VERSION,
};

fn fixtures_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2) // crates/video-state -> workspace root
        .expect("workspace root")
        .join("fixtures/migrations/v1-to-v2")
}

#[test]
fn frozen_plan_matches_documented_step_set() {
    let plan = v1_to_v2_plan();
    assert_eq!(plan.from, FROM_VERSION);
    assert_eq!(plan.to, TO_VERSION);
    assert_eq!(plan.steps.len(), FROZEN_STEPS.len());
    for (i, step) in plan.steps.iter().enumerate() {
        assert_eq!(step.step as usize, i + 1, "step number is contiguous");
    }
    assert_eq!(plan.backup_count, 3);
}

#[test]
fn every_frozen_step_keeps_a_serializable_form() {
    for (i, (name, _desc, _backup, fields)) in FROZEN_STEPS.iter().enumerate() {
        let frozen = FrozenStep {
            from: FROM_VERSION.to_string(),
            to: TO_VERSION.to_string(),
            step: (i + 1) as u32,
            name: (*name).to_string(),
            requires_backup: *_backup,
            touched_fields: fields.iter().map(|s| (*s).to_string()).collect(),
            description: "round-trip".to_string(),
        };
        let expected_fields = frozen.touched_fields.clone();
        let step: MigrationStep = frozen.into();
        assert_eq!(step.name, *name);
        assert_eq!(step.touched_fields, expected_fields);
    }
}

#[test]
fn on_disk_descriptors_match_frozen_plan() {
    let dir = fixtures_root();
    let mut found = 0;
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json")
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(char::is_numeric)
        {
            let step: MigrationStep = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            assert_eq!(step.from, FROM_VERSION);
            assert_eq!(step.to, TO_VERSION);
            found += 1;
        }
    }
    assert_eq!(found, FROZEN_STEPS.len());
}

#[test]
fn migration_runner_consumes_v1_to_v2_plan() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let fixtures = fixtures_root();
    let migrations_dir = dir.join("migrations/v1-to-v2");
    fs::create_dir_all(&migrations_dir).unwrap();
    for entry in fs::read_dir(fixtures).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let bytes = fs::read(&path).unwrap();
            fs::write(migrations_dir.join(path.file_name().unwrap()), bytes).unwrap();
        }
    }
    let runner = MigrationRunner::new(dir);
    let plan = runner.plan(FROM_VERSION, TO_VERSION).expect("plan");
    assert_eq!(plan.steps.len(), FROZEN_STEPS.len());
    assert_eq!(plan.backup_count, 3);
    let dry = runner.dry_run(&plan).expect("dry-run");
    assert_eq!(dry, plan);
}
