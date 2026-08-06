//! CutRight v2 skill monitor core.
//!
//! Read-only health auditor for the local skills tree. Reports typed
//! `healthy` / `degraded` / `failed` states per skill without modifying
//! anything. Concept adapted from the workspace bounded-run monitor
//! qualification tooling (workspace pin
//! 6ee21f03a787e7b57dc412760a8996ea7a235302, user-owned), reduced to
//! project-scoped, local-only supervision: no workspace-global agent state.
//!
//! States and reason codes are stable strings; consumers (gates, receipts)
//! match on them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

pub const MONITOR_SCHEMA_VERSION: i64 = 1;

/// Failed reason codes.
pub const R_MISSING_SKILL_MD: &str = "missing_skill_md";
pub const R_INVALID_FRONTMATTER: &str = "invalid_frontmatter";
pub const R_INVALID_ID: &str = "invalid_id";
pub const R_INVALID_VERSION: &str = "invalid_version";
pub const R_DANGLING_DEPENDENCY: &str = "dangling_dependency";
pub const R_DEPENDENCY_CYCLE: &str = "dependency_cycle";
pub const R_UNDECLARED_PERMISSION: &str = "undeclared_permission";
pub const R_EXTERNAL_PATH_RESOURCE: &str = "external_path_resource";
pub const R_MISSING_RESOURCE: &str = "missing_resource";

/// Degraded reason codes.
pub const R_MISSING_DESCRIPTION: &str = "missing_description";
pub const R_MISSING_NOTICE: &str = "missing_notice";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillState {
    Healthy,
    Degraded,
    Failed,
}

impl SkillState {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillState::Healthy => "healthy",
            SkillState::Degraded => "degraded",
            SkillState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillStatus {
    pub id: String,
    pub state: SkillState,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RootStatus {
    Present,
    Missing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorReport {
    pub root_status: RootStatus,
    pub registry_present: bool,
    pub skills: Vec<SkillStatus>,
}

impl MonitorReport {
    pub fn summary(&self) -> (usize, usize, usize) {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut failed = 0;
        for skill in &self.skills {
            match skill.state {
                SkillState::Healthy => healthy += 1,
                SkillState::Degraded => degraded += 1,
                SkillState::Failed => failed += 1,
            }
        }
        (healthy, degraded, failed)
    }

    pub fn has_failures(&self) -> bool {
        self.summary().2 > 0
    }
}

#[derive(Debug, Default)]
struct Declaration {
    parsed: bool,
    id: Option<String>,
    version: Option<String>,
    description: Option<String>,
    depends: Vec<String>,
    permissions: Vec<String>,
    resources: Vec<String>,
}

pub fn monitor(root: &Path) -> MonitorReport {
    let skills_root = root.join("skills");
    if !skills_root.is_dir() {
        return MonitorReport {
            root_status: RootStatus::Missing,
            registry_present: false,
            skills: Vec::new(),
        };
    }
    let capabilities = load_capabilities(root);
    let registry_present = root.join("capabilities").join("registry.json").is_file();

    let mut names: Vec<String> = fs::read_dir(&skills_root)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    names.sort();

    let mut declarations: BTreeMap<String, Declaration> = BTreeMap::new();
    for name in &names {
        declarations.insert(name.clone(), parse_declaration(&skills_root.join(name)));
    }
    let cyclic = cyclic_members(&names, &declarations, &skills_root);

    let mut skills = Vec::new();
    for name in &names {
        let decl = &declarations[name];
        let mut failed: Vec<String> = Vec::new();
        let mut degraded: Vec<String> = Vec::new();
        let skill_dir = skills_root.join(name);

        if !decl.parsed {
            failed.push(format!("{R_MISSING_SKILL_MD}"));
        } else {
            if decl.id.is_none() || decl.version.is_none() {
                failed.push(format!("{R_INVALID_FRONTMATTER}"));
            }
            match decl.id.as_deref() {
                Some(id) if id == name && is_skill_id(id) => {}
                _ => failed.push(format!("{R_INVALID_ID}")),
            }
            if !decl
                .version
                .as_deref()
                .map(is_version)
                .unwrap_or(false)
            {
                failed.push(format!("{R_INVALID_VERSION}"));
            }
            for dep in &decl.depends {
                if !skills_root.join(dep).is_dir() {
                    failed.push(format!("{R_DANGLING_DEPENDENCY}:{dep}"));
                }
            }
            if cyclic.contains(name.as_str()) {
                failed.push(format!("{R_DEPENDENCY_CYCLE}"));
            }
            for permission in &decl.permissions {
                if !capabilities.contains(permission) {
                    failed.push(format!("{R_UNDECLARED_PERMISSION}:{permission}"));
                }
            }
            for rel in &decl.resources {
                if resource_path_is_external(rel) {
                    failed.push(format!("{R_EXTERNAL_PATH_RESOURCE}:{rel}"));
                } else if !skill_dir.join(rel).is_file() {
                    failed.push(format!("{R_MISSING_RESOURCE}:{rel}"));
                }
            }
            if decl
                .description
                .as_deref()
                .map(str::trim)
                .map(str::is_empty)
                .unwrap_or(true)
            {
                degraded.push(format!("{R_MISSING_DESCRIPTION}"));
            }
            if !decl.resources.is_empty() && !skill_dir.join("NOTICE.md").is_file() {
                degraded.push(format!("{R_MISSING_NOTICE}"));
            }
        }

        failed.sort();
        degraded.sort();
        let (state, reasons) = if !failed.is_empty() {
            (SkillState::Failed, failed)
        } else if !degraded.is_empty() {
            (SkillState::Degraded, degraded)
        } else {
            (SkillState::Healthy, Vec::new())
        };
        skills.push(SkillStatus {
            id: name.clone(),
            state,
            reasons,
        });
    }

    MonitorReport {
        root_status: RootStatus::Present,
        registry_present,
        skills,
    }
}

fn parse_declaration(skill_dir: &Path) -> Declaration {
    let mut decl = Declaration::default();
    let text = match fs::read_to_string(skill_dir.join("SKILL.md")) {
        Ok(text) => text,
        Err(_) => return decl,
    };
    decl.parsed = true;
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().copied().map(str::trim) != Some("---") {
        decl.id = None;
        decl.version = None;
        return decl;
    }
    let mut end = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end = Some(index);
            break;
        }
    }
    let end = match end {
        Some(end) => end,
        None => {
            decl.id = None;
            decl.version = None;
            return decl;
        }
    };
    for line in &lines[1..end] {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key.trim() {
                "id" => decl.id = Some(value.to_string()),
                "version" => decl.version = Some(value.to_string()),
                "description" => decl.description = Some(value.to_string()),
                "depends" => decl.depends = split_list(value),
                "permissions" => decl.permissions = split_list(value),
                "resources" => decl.resources = split_list(value),
                _ => {}
            }
        }
    }
    decl
}

fn split_list(value: &str) -> Vec<String> {
    if value.eq_ignore_ascii_case("none") || value.is_empty() {
        return Vec::new();
    }
    let mut items: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    items.sort();
    items.dedup();
    items
}

fn is_skill_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.chars().all(|c| c.is_ascii_digit())
                && (*part == "0" || !part.starts_with('0'))
        })
}

fn resource_path_is_external(rel: &str) -> bool {
    if rel.is_empty()
        || rel.contains("://")
        || rel.starts_with('/')
        || rel.starts_with('\\')
        || rel.contains('\\')
        || rel.contains('*')
        || rel.contains('?')
    {
        return true;
    }
    Path::new(rel)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
}

/// Tolerant capability-name extraction from `capabilities/registry.json`:
/// collects every string value bound to a `"name"` key.
fn load_capabilities(root: &Path) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let path = root.join("capabilities").join("registry.json");
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return set,
    };
    let bytes = text.as_bytes();
    let mut index = 0;
    while let Some(found) = text[index..].find("\"name\"") {
        index += found + "\"name\"".len();
        let mut cursor = index;
        while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t' | b'\n' | b'\r') {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b':' {
            continue;
        }
        cursor += 1;
        while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t' | b'\n' | b'\r') {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'"' {
            continue;
        }
        cursor += 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'"' {
            if bytes[cursor] == b'\\' {
                cursor += 1;
            }
            cursor += 1;
        }
        if cursor < bytes.len() {
            if let Ok(value) = std::str::from_utf8(&bytes[start..cursor]) {
                if !value.is_empty() {
                    set.insert(value.to_string());
                }
            }
            index = cursor + 1;
        } else {
            break;
        }
    }
    set
}

/// Members of dependency cycles (Kahn peeling over declared dependencies).
fn cyclic_members(
    names: &[String],
    declarations: &BTreeMap<String, Declaration>,
    skills_root: &Path,
) -> BTreeSet<String> {
    let mut indegree: BTreeMap<&str, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut present: BTreeSet<&str> = BTreeSet::new();
    for name in names {
        present.insert(name.as_str());
        indegree.entry(name.as_str()).or_insert(0);
    }
    for name in names {
        for dep in &declarations[name].depends {
            if present.contains(dep.as_str()) {
                *indegree.entry(name.as_str()).or_insert(0) += 1;
                dependents.entry(dep.as_str()).or_default().push(name.as_str());
            }
        }
    }
    let mut ready: Vec<&str> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut peeled: BTreeSet<&str> = BTreeSet::new();
    while let Some(next) = ready.pop() {
        peeled.insert(next);
        if let Some(children) = dependents.get(next) {
            for child in children {
                if let Some(degree) = indegree.get_mut(child) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 && !peeled.contains(child) {
                        ready.push(child);
                    }
                }
            }
        }
    }
    let _ = skills_root;
    present
        .into_iter()
        .filter(|id| !peeled.contains(id))
        .map(str::to_string)
        .collect()
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Deterministic canonical JSON report: sorted skills, no timestamps.
pub fn render_report(report: &MonitorReport) -> String {
    let (healthy, degraded, failed) = report.summary();
    let mut out = String::new();
    out.push('{');
    out.push_str(&format!(
        "\"registry_present\":{},",
        report.registry_present
    ));
    out.push_str(&format!(
        "\"root_status\":{},",
        escape(match report.root_status {
            RootStatus::Present => "present",
            RootStatus::Missing => "missing",
        })
    ));
    out.push_str("\"schema_version\":1,");
    out.push('"');
    out.push_str("skills");
    out.push_str("\":[");
    for (index, skill) in report.skills.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"id\":{},\"reasons\":[{}],\"state\":{}}}",
            escape(&skill.id),
            skill
                .reasons
                .iter()
                .map(|reason| escape(reason))
                .collect::<Vec<_>>()
                .join(","),
            escape(skill.state.as_str())
        ));
    }
    out.push_str("],");
    out.push_str(&format!(
        "\"summary\":{{\"degraded\":{degraded},\"failed\":{failed},\"healthy\":{healthy}}}"
    ));
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::{is_skill_id, is_version, resource_path_is_external};

    #[test]
    fn id_and_version_rules() {
        assert!(is_skill_id("content-video-editor"));
        assert!(!is_skill_id("Bad_Name"));
        assert!(!is_skill_id(""));
        assert!(is_version("1.0.0"));
        assert!(is_version("0.1.0"));
        assert!(!is_version("1.0"));
        assert!(!is_version("01.0.0"));
        assert!(!is_version("1.0.x"));
    }

    #[test]
    fn external_path_detection() {
        assert!(resource_path_is_external("../x.md"));
        assert!(resource_path_is_external("/etc/passwd"));
        assert!(resource_path_is_external("a\\b.md"));
        assert!(resource_path_is_external("*.md"));
        assert!(resource_path_is_external("https://x/y"));
        assert!(!resource_path_is_external("references/rules.md"));
    }
}
