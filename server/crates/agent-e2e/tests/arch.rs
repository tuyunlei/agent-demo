use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;
use walkdir::WalkDir;

fn server_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn forbidden_deps() -> HashMap<&'static str, HashSet<&'static str>> {
    HashMap::from([
        (
            "agent-types",
            HashSet::from([
                "agent-app",
                "agent-channel",
                "agent-server",
                "agent-storage",
                "agent-llm",
                "agent-proto",
                "agent-domain",
            ]),
        ),
        (
            "agent-domain",
            HashSet::from([
                "agent-app",
                "agent-channel",
                "agent-server",
                "agent-storage",
                "agent-llm",
                "agent-proto",
            ]),
        ),
        (
            "agent-app",
            HashSet::from([
                "agent-channel",
                "agent-server",
                "agent-storage",
                "agent-llm",
                "agent-proto",
            ]),
        ),
        (
            "agent-proto",
            HashSet::from([
                "agent-domain",
                "agent-app",
                "agent-channel",
                "agent-server",
                "agent-storage",
                "agent-llm",
            ]),
        ),
        (
            "agent-storage",
            HashSet::from([
                "agent-app",
                "agent-channel",
                "agent-server",
                "agent-llm",
                "agent-proto",
            ]),
        ),
        (
            "agent-llm",
            HashSet::from([
                "agent-app",
                "agent-channel",
                "agent-server",
                "agent-storage",
                "agent-proto",
            ]),
        ),
    ])
}

fn collect_agent_deps(doc: &Value) -> HashSet<String> {
    let mut deps = HashSet::new();

    for section in ["dependencies", "dev-dependencies"] {
        if let Some(table) = doc.get(section).and_then(Value::as_table) {
            for dep_name in table.keys() {
                if dep_name.starts_with("agent-") {
                    deps.insert(dep_name.to_string());
                }
            }
        }
    }

    deps
}

#[test]
fn architecture_dependency_directions_are_valid() {
    let crates_dir = server_root().join("crates");
    let forbidden = forbidden_deps();
    let mut violations = Vec::new();

    for entry in fs::read_dir(&crates_dir).expect("failed to read crates directory") {
        let entry = entry.expect("failed to read crates entry");
        if !entry
            .file_type()
            .expect("failed to read file type")
            .is_dir()
        {
            continue;
        }

        let crate_name = entry.file_name().to_string_lossy().to_string();
        if crate_name == "agent-e2e" {
            continue;
        }

        let cargo_toml = entry.path().join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }

        let content = fs::read_to_string(&cargo_toml)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", cargo_toml.display()));
        let parsed: Value = content
            .parse()
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", cargo_toml.display()));

        let deps = collect_agent_deps(&parsed);

        if let Some(forbidden_set) = forbidden.get(crate_name.as_str()) {
            for dep in deps {
                if forbidden_set.contains(dep.as_str()) {
                    violations.push(format!(
                        "Architecture violation: '{}' must not depend on '{}' (found in {})",
                        crate_name,
                        dep,
                        cargo_toml
                            .strip_prefix(server_root())
                            .unwrap_or(&cargo_toml)
                            .display()
                    ));
                }
            }
        }
    }

    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

fn effective_line_count(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}

#[test]
fn rust_file_sizes_are_within_limits() {
    let crates_dir = server_root().join("crates");
    let mut violations = Vec::new();

    for entry in WalkDir::new(&crates_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        if path
            .components()
            .any(|component| component.as_os_str().to_string_lossy() == "target")
        {
            continue;
        }

        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|name| name.ends_with(".pb.rs"))
        {
            continue;
        }

        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let effective_lines = effective_line_count(&content);

        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let max_lines = if file_name.ends_with("_tests.rs") {
            500
        } else {
            300
        };

        if effective_lines > max_lines {
            let display_path = path.strip_prefix(server_root()).unwrap_or(path).display();
            violations.push(format!(
                "File too large: {} has {} effective lines (max {})",
                display_path, effective_lines, max_lines
            ));
        }
    }

    assert!(violations.is_empty(), "{}", violations.join("\n"));
}
