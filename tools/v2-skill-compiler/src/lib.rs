//! CutRight v2 skill compiler core.
//!
//! Consumes only three inputs under `--root`:
//!   - `skills/`            — skill directories, each with a `SKILL.md`
//!   - `schemas/skills/`    — skill schema documents (must parse as JSON)
//!   - `capabilities/registry.json` — capability registry granting permissions
//!
//! Produces a deterministic, byte-identical skill pack plus a topology
//! report. Rejects external paths, dangling dependencies, undeclared
//! permissions, mutable resource references, and dependency cycles.
//!
//! Concept adapted from the workspace bounded-run skill compiler (workspace
//! pin 6ee21f03a787e7b57dc412760a8996ea7a235302, user-owned); reimplemented
//! for the CutRight-local skill model with zero external dependencies.

pub mod json;
pub mod sha256;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use json::Value;

pub const PACK_SCHEMA_VERSION: i64 = 1;
pub const PACK_ID: &str = "cutright-skill-pack-v1";
pub const SKILL_MD: &str = "SKILL.md";

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRef {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledSkill {
    pub id: String,
    pub version: String,
    pub description: String,
    pub content_hash: String,
    pub dependencies: Vec<String>,
    pub permissions: Vec<String>,
    pub resources: Vec<ResourceRef>,
    pub order: usize,
}

#[derive(Debug, Default)]
pub struct CompileReport {
    pub skills: Vec<CompiledSkill>,
    pub schemas: Vec<(String, String)>, // (relative path, sha256)
    pub capability_count: usize,
    pub registry_present: bool,
    pub topology_order: Vec<String>,
    pub errors: Vec<String>,
}

impl CompileReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Read-only view of one skill directory's declarations.
#[derive(Debug, Default)]
struct Declared {
    id: Option<String>,
    version: Option<String>,
    description: Option<String>,
    depends: Vec<String>,
    permissions: Vec<String>,
    resources: Vec<String>,
    parse_errors: Vec<String>,
}

pub fn compile(root: &Path) -> CompileReport {
    let mut report = CompileReport::default();
    let skills_root = root.join("skills");
    if !skills_root.is_dir() {
        report
            .errors
            .push(format!("missing skills root: {}", skills_root.display()));
        return report;
    }

    let capabilities = load_capabilities(root, &mut report);
    report.capability_count = capabilities.len();
    load_schemas(root, &mut report);

    let mut skill_dirs: Vec<String> = Vec::new();
    match fs::read_dir(&skills_root) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        skill_dirs.push(name.to_string());
                    }
                }
            }
        }
        Err(err) => {
            report
                .errors
                .push(format!("unreadable skills root: {err}"));
            return report;
        }
    }
    skill_dirs.sort();

    let mut declared: BTreeMap<String, Declared> = BTreeMap::new();
    for name in &skill_dirs {
        declared.insert(name.clone(), parse_skill(&skills_root, name, &mut report));
    }

    let mut compiled: BTreeMap<String, CompiledSkill> = BTreeMap::new();
    for name in &skill_dirs {
        let decl = &declared[name];
        if decl.id.is_none() || decl.version.is_none() {
            continue; // parse errors already recorded
        }
        let errors_before = report.errors.len();
        validate_declaration(name, decl, &capabilities, &skills_root, &mut report);
        if report.errors.len() > errors_before {
            continue;
        }
        let mut resources = Vec::new();
        let mut resource_failed = false;
        for rel in &decl.resources {
            match read_resource(&skills_root.join(name), rel) {
                Ok(resource) => resources.push(resource),
                Err(err) => {
                    report.errors.push(format!("[{name}] {err}"));
                    resource_failed = true;
                }
            }
        }
        if resource_failed {
            continue;
        }
        resources.sort_by(|a, b| a.path.cmp(&b.path));
        let skill_md = skills_root.join(name).join(SKILL_MD);
        let body = match fs::read(&skill_md) {
            Ok(bytes) => bytes,
            Err(err) => {
                report
                    .errors
                    .push(format!("[{name}] unreadable {SKILL_MD}: {err}"));
                continue;
            }
        };
        compiled.insert(
            name.clone(),
            CompiledSkill {
                id: name.clone(),
                version: decl.version.clone().unwrap_or_default(),
                description: decl.description.clone().unwrap_or_default(),
                content_hash: format!("sha256:{}", sha256::sha256_hex(&body)),
                dependencies: decl.depends.clone(),
                permissions: decl.permissions.clone(),
                resources,
                order: 0,
            },
        );
    }

    let order = match topological_order(&compiled) {
        Ok(order) => order,
        Err(cycle) => {
            report.errors.push(format!("dependency cycle: {}", cycle.join(" -> ")));
            return report;
        }
    };
    for (index, id) in order.iter().enumerate() {
        if let Some(skill) = compiled.get_mut(id) {
            skill.order = index;
        }
    }
    report.topology_order = order;
    report.skills = compiled.into_values().collect();
    report.skills.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    report
}

fn load_capabilities(root: &Path, report: &mut CompileReport) -> BTreeSet<String> {
    let registry_path = root.join("capabilities").join("registry.json");
    let mut set = BTreeSet::new();
    match fs::read_to_string(&registry_path) {
        Ok(text) => {
            report.registry_present = true;
            match json::parse(&text) {
                Ok(value) => {
                    if let Some(items) = value.get("capabilities").and_then(Value::as_array) {
                        for item in items {
                            let name = match item {
                                Value::String(name) => Some(name.clone()),
                                Value::Object(_) => item
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .map(|s| s.to_string()),
                                _ => None,
                            };
                            match name {
                                Some(name) if !name.is_empty() => {
                                    set.insert(name);
                                }
                                _ => report.errors.push(format!(
                                    "capabilities/registry.json entry without a name: {}",
                                    json::canonical(item)
                                )),
                            }
                        }
                    } else {
                        report.errors.push(
                            "capabilities/registry.json missing `capabilities` array".into(),
                        );
                    }
                }
                Err(err) => report
                    .errors
                    .push(format!("capabilities/registry.json invalid JSON: {err}")),
            }
        }
        Err(_) => {
            // Absent registry is allowed: no permission may be granted.
            report.registry_present = false;
        }
    }
    set
}

fn load_schemas(root: &Path, report: &mut CompileReport) {
    let schemas_root = root.join("schemas").join("skills");
    if !schemas_root.is_dir() {
        return; // absent schema root means no schema constraints
    }
    let mut files: Vec<PathBuf> = Vec::new();
    collect_files(&schemas_root, &mut files);
    files.sort();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        match fs::read(&path) {
            Ok(bytes) => {
                match std::str::from_utf8(&bytes) {
                    Ok(text) => {
                        if let Err(err) = json::parse(text) {
                            report.errors.push(format!("{rel} invalid JSON: {err}"));
                            continue;
                        }
                    }
                    Err(_) => {
                        report.errors.push(format!("{rel} is not valid UTF-8"));
                        continue;
                    }
                }
                report
                    .schemas
                    .push((rel, format!("sha256:{}", sha256::sha256_hex(&bytes))));
            }
            Err(err) => report.errors.push(format!("{rel} unreadable: {err}")),
        }
    }
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files(&path, out);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
}

fn parse_skill(skills_root: &Path, name: &str, report: &mut CompileReport) -> Declared {
    let mut decl = Declared::default();
    let path = skills_root.join(name).join(SKILL_MD);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            report
                .errors
                .push(format!("[{name}] missing or unreadable {SKILL_MD}"));
            return decl;
        }
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.first().copied().map(str::trim) != Some("---") {
        report
            .errors
            .push(format!("[{name}] {SKILL_MD} must begin with a `---` front-matter block"));
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
            report
                .errors
                .push(format!("[{name}] unterminated front-matter block"));
            return decl;
        }
    };
    for line in &lines[1..end] {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = match line.split_once(':') {
            Some(pair) => pair,
            None => {
                decl.parse_errors
                    .push(format!("malformed front-matter line: {line}"));
                continue;
            }
        };
        let value = value.trim();
        match key.trim() {
            "id" => decl.id = Some(value.to_string()),
            "version" => decl.version = Some(value.to_string()),
            "description" => decl.description = Some(value.to_string()),
            "depends" => decl.depends = split_list(value),
            "permissions" => decl.permissions = split_list(value),
            "resources" => decl.resources = split_list(value),
            other => decl
                .parse_errors
                .push(format!("unknown front-matter key: {other}")),
        }
    }
    for err in &decl.parse_errors {
        report.errors.push(format!("[{name}] {err}"));
    }
    if decl.id.is_none() {
        report.errors.push(format!("[{name}] front matter missing `id`"));
    }
    if decl.version.is_none() {
        report.errors.push(format!("[{name}] front matter missing `version`"));
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

fn validate_declaration(
    name: &str,
    decl: &Declared,
    capabilities: &BTreeSet<String>,
    skills_root: &Path,
    report: &mut CompileReport,
) {
    let id = decl.id.as_deref().unwrap_or("");
    if id != name {
        report.errors.push(format!(
            "[{name}] declared id `{id}` does not match directory name"
        ));
    }
    if !is_skill_id(id) {
        report
            .errors
            .push(format!("[{name}] invalid skill id `{id}` (must match ^[a-z0-9-]+$)"));
    }
    let version = decl.version.as_deref().unwrap_or("");
    if !is_version(version) {
        report.errors.push(format!(
            "[{name}] invalid version `{version}` (must be MAJOR.MINOR.PATCH digits)"
        ));
    }
    for dep in &decl.depends {
        if !is_skill_id(dep) {
            report
                .errors
                .push(format!("[{name}] invalid dependency id `{dep}`"));
        }
        if !skills_root.join(dep).is_dir() {
            report
                .errors
                .push(format!("[{name}] dangling dependency `{dep}`"));
        }
    }
    for permission in &decl.permissions {
        if !capabilities.contains(permission) {
            report.errors.push(format!(
                "[{name}] undeclared permission `{permission}` (absent from capabilities/registry.json)"
            ));
        }
    }
    for rel in &decl.resources {
        if let Err(err) = check_resource_path(rel) {
            report.errors.push(format!("[{name}] {err}"));
        }
    }
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

/// A resource path must stay inside the skill directory: relative, no parent
/// traversal, no absolute forms, no glob/mutable forms, no URLs.
fn check_resource_path(rel: &str) -> Result<(), String> {
    if rel.is_empty() {
        return Err("empty resource path".into());
    }
    if rel.contains("://") {
        return Err(format!("external URL resource rejected: {rel}"));
    }
    if rel.starts_with('/') || rel.starts_with('\\') {
        return Err(format!("absolute resource path rejected: {rel}"));
    }
    if rel.contains('\\') {
        return Err(format!("non-portable resource path rejected: {rel}"));
    }
    if rel.contains('*') || rel.contains('?') {
        return Err(format!("mutable/glob resource reference rejected: {rel}"));
    }
    for component in Path::new(rel).components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(format!("external resource path rejected: {rel}")),
        }
    }
    Ok(())
}

fn read_resource(skill_dir: &Path, rel: &str) -> Result<ResourceRef, String> {
    check_resource_path(rel)?;
    let path = skill_dir.join(rel);
    let resolved = path.canonicalize().map_err(|err| {
        format!("resource `{rel}` does not resolve to an existing file: {err}")
    })?;
    let skill_dir_resolved = skill_dir
        .canonicalize()
        .map_err(|err| format!("skill directory unresolvable: {err}"))?;
    if !resolved.starts_with(&skill_dir_resolved) {
        return Err(format!("resource `{rel}` escapes the skill directory"));
    }
    let bytes = fs::read(&resolved)
        .map_err(|err| format!("resource `{rel}` unreadable: {err}"))?;
    Ok(ResourceRef {
        path: rel.to_string(),
        sha256: format!("sha256:{}", sha256::sha256_hex(&bytes)),
        bytes: bytes.len() as u64,
    })
}

/// Deterministic Kahn topological order with lexicographic tie-breaking.
fn topological_order(skills: &BTreeMap<String, CompiledSkill>) -> Result<Vec<String>, Vec<String>> {
    let mut indegree: BTreeMap<&str, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (id, skill) in skills {
        indegree.entry(id.as_str()).or_insert(0);
        for dep in &skill.dependencies {
            if skills.contains_key(dep) {
                *indegree.entry(id.as_str()).or_insert(0) += 1;
                dependents.entry(dep.as_str()).or_default().push(id.as_str());
            }
        }
    }
    let mut ready: BTreeSet<&str> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut order = Vec::new();
    while let Some(next) = ready.iter().next().copied() {
        ready.remove(next);
        order.push(next.to_string());
        if let Some(children) = dependents.get(next) {
            for child in children {
                let degree = indegree.get_mut(child).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child);
                }
            }
        }
    }
    if order.len() != skills.len() {
        let mut cycle: Vec<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree > 0)
            .map(|(id, _)| (*id).to_string())
            .collect();
        cycle.sort();
        return Err(cycle);
    }
    Ok(order)
}

fn value_string(value: &str) -> Value {
    Value::String(value.to_string())
}

fn value_string_list(items: &[String]) -> Value {
    Value::Array(items.iter().map(|s| value_string(s)).collect())
}

/// Canonical pack JSON. Deterministic: sorted keys, no timestamps, no
/// environment-dependent values. Two identical inputs produce byte-identical
/// output.
pub fn render_pack(report: &CompileReport) -> String {
    let mut skills = Vec::new();
    for skill in &report.skills {
        let mut entry = BTreeMap::new();
        entry.insert("id".into(), value_string(&skill.id));
        entry.insert("version".into(), value_string(&skill.version));
        entry.insert("description".into(), value_string(&skill.description));
        entry.insert("content_hash".into(), value_string(&skill.content_hash));
        entry.insert("dependencies".into(), value_string_list(&skill.dependencies));
        entry.insert("permissions".into(), value_string_list(&skill.permissions));
        entry.insert(
            "resources".into(),
            Value::Array(
                skill
                    .resources
                    .iter()
                    .map(|resource| {
                        let mut map = BTreeMap::new();
                        map.insert("path".into(), value_string(&resource.path));
                        map.insert("sha256".into(), value_string(&resource.sha256));
                        map.insert("bytes".into(), Value::Integer(resource.bytes as i64));
                        Value::Object(map)
                    })
                    .collect(),
            ),
        );
        entry.insert("order".into(), Value::Integer(skill.order as i64));
        skills.push(Value::Object(entry));
    }
    let skills_value = Value::Array(skills);
    let pack_hash = format!("sha256:{}", sha256::sha256_hex(json::canonical(&skills_value).as_bytes()));

    let mut root_map = BTreeMap::new();
    root_map.insert("schema_version".into(), Value::Integer(PACK_SCHEMA_VERSION));
    root_map.insert("pack_id".into(), value_string(PACK_ID));
    root_map.insert(
        "registry_present".into(),
        Value::Bool(report.registry_present),
    );
    root_map.insert(
        "capability_count".into(),
        Value::Integer(report.capability_count as i64),
    );
    root_map.insert(
        "schemas".into(),
        Value::Array(
            report
                .schemas
                .iter()
                .map(|(path, hash)| {
                    let mut map = BTreeMap::new();
                    map.insert("path".into(), value_string(path));
                    map.insert("sha256".into(), value_string(hash));
                    Value::Object(map)
                })
                .collect(),
        ),
    );
    root_map.insert("skills".into(), skills_value);
    root_map.insert("pack_hash".into(), value_string(&pack_hash));
    json::canonical(&Value::Object(root_map))
}

/// Topology report JSON: nodes, edges, deterministic order.
pub fn render_topology(report: &CompileReport) -> String {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut by_id: BTreeMap<&str, &CompiledSkill> = BTreeMap::new();
    for skill in &report.skills {
        by_id.insert(&skill.id, skill);
    }
    for skill in report.skills.iter() {
        nodes.push(value_string(&skill.id));
        for dep in &skill.dependencies {
            let mut edge = BTreeMap::new();
            edge.insert("from".into(), value_string(&skill.id));
            edge.insert("to".into(), value_string(dep));
            edges.push(Value::Object(edge));
        }
    }
    edges.sort_by(|a, b| json::canonical(a).cmp(&json::canonical(b)));
    let mut map = BTreeMap::new();
    map.insert("schema_version".into(), Value::Integer(PACK_SCHEMA_VERSION));
    map.insert("nodes".into(), Value::Array(nodes));
    map.insert("edges".into(), Value::Array(edges));
    map.insert(
        "order".into(),
        Value::Array(report.topology_order.iter().map(|s| value_string(s)).collect()),
    );
    map.insert(
        "cyclic".into(),
        Value::Bool(report.topology_order.len() != report.skills.len()),
    );
    json::canonical(&Value::Object(map))
}
