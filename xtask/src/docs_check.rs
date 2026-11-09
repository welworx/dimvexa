// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask check-docs`: keeps the contributor documentation, the agent skills and the
//! workflows honest. It checks, with plain text matching and no dependencies:
//!
//! * every `cargo xtask <command>` named in `CONTRIBUTING.md`, `AGENTS.md`, `CLAUDE.md` and the
//!   skills exists, and every stage named after `cargo xtask ci` is a real stage;
//! * every path under a known top-level directory that those files name in backticks exists;
//! * `CONTRIBUTING.md` names every stage of `cargo xtask ci`;
//! * the skills table of `AGENTS.md` equals the folders of `.claude/skills/`, and each
//!   `SKILL.md` has front matter whose `name` is its folder name and a `description`;
//! * workflows: every job has `timeout-minutes`, none uses `pull_request_target` or a `@master`
//!   action reference;
//! * no scratch or probe files, no blanket lint allowance and no unresolved owner marker is
//!   committed.

use std::fs;
use std::path::{Path, PathBuf};

use crate::ci;

/// Commands `cargo xtask` understands (see `main.rs`).
const COMMANDS: &[&str] = &[
    "ci",
    "validate-data",
    "update-data-hash",
    "check-docs",
    "bench-check",
];

/// Top-level directories whose paths are checked for existence when named in backticks.
const ROOTS: &[&str] = &[
    "crates/",
    "docs/",
    ".github/",
    ".claude/",
    "xtask/",
    "extras/",
    "fuzz/",
    "examples/",
    "scripts/",
    "benchmarks/",
    "bindings/",
];

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Text between pairs of single backticks (inline code), one per match.
fn inline_code(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            continue;
        }
        let parts: Vec<&str> = line.split('`').collect();
        for (i, p) in parts.iter().enumerate() {
            if i % 2 == 1 && i.saturating_add(1) < parts.len() {
                out.push((*p).to_string());
            }
        }
    }
    out
}

/// All commands that appear in code spans or fenced blocks starting with `cargo xtask`.
fn xtask_invocations(text: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut candidates = inline_code(text);
    candidates.extend(
        text.lines()
            .map(|l| l.trim().trim_start_matches("$ ").to_string()),
    );
    for c in candidates {
        if let Some(rest) = c.trim().strip_prefix("cargo xtask ") {
            // Stop at a comment or a pipe.
            let rest = rest.split(['#', '|']).next().unwrap_or("");
            out.push(rest.split_whitespace().map(String::from).collect());
        }
    }
    out
}

fn skill_dirs(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join(".claude/skills"))
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn front_matter(text: &str) -> Option<Vec<(String, String)>> {
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    Some(
        body[..end]
            .lines()
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect(),
    )
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.filter_map(Result::ok) {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if !matches!(name.as_str(), "target" | ".git" | "node_modules") {
                files_under(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// Returns the list of problems (empty when everything is consistent).
pub fn run(root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let mut docs: Vec<PathBuf> = ["CONTRIBUTING.md", "AGENTS.md", "CLAUDE.md"]
        .iter()
        .map(|f| root.join(f))
        .collect();
    for s in skill_dirs(root) {
        docs.push(root.join(".claude/skills").join(s).join("SKILL.md"));
    }
    let stage_names = ci::stage_names();

    for doc in &docs {
        let rel = doc.strip_prefix(root).unwrap_or(doc).display().to_string();
        if !doc.is_file() {
            errors.push(format!("{rel}: file is missing"));
            continue;
        }
        let text = read(doc);
        for inv in xtask_invocations(&text) {
            let Some(cmd) = inv.first() else { continue };
            if !COMMANDS.contains(&cmd.as_str()) {
                errors.push(format!(
                    "{rel}: `cargo xtask {cmd}` is not an xtask command"
                ));
            }
            if cmd == "ci" {
                for arg in inv.iter().skip(1).filter(|a| !a.starts_with('-')) {
                    if !stage_names.contains(&arg.as_str()) {
                        errors.push(format!("{rel}: `cargo xtask ci {arg}`: no such stage"));
                    }
                }
            }
        }
        for token in inline_code(&text) {
            let t = token.trim();
            let looks_like_path = ROOTS.iter().any(|r| t.starts_with(r))
                && !t.contains([' ', '*', '<', '>', '$', '{', '}'])
                && !t.ends_with("...");
            if looks_like_path && !root.join(t).exists() {
                errors.push(format!("{rel}: path `{t}` does not exist"));
            }
        }
    }

    // CONTRIBUTING names every stage.
    let contributing = read(&root.join("CONTRIBUTING.md"));
    let named = inline_code(&contributing);
    for stage in &stage_names {
        if !named.iter().any(|n| n == stage) {
            errors.push(format!(
                "CONTRIBUTING.md: stage `{stage}` of `cargo xtask ci` is not listed"
            ));
        }
    }

    // Skills table equals the skill folders; front matter is sound.
    let dirs = skill_dirs(root);
    let agents = read(&root.join("AGENTS.md"));
    let mut table: Vec<String> = agents
        .lines()
        .filter(|l| l.starts_with("| `"))
        .filter_map(|l| l.split('`').nth(1))
        .filter(|n| dirs.iter().any(|d| d == n) || !n.contains(['/', '.']))
        .map(String::from)
        .collect();
    table.retain(|n| dirs.contains(n) || !n.contains('/'));
    table.sort();
    let expected: Vec<String> = dirs.clone();
    let in_table: Vec<String> = table.into_iter().filter(|n| expected.contains(n)).collect();
    if in_table != expected {
        errors.push(format!(
            "AGENTS.md: skills table {in_table:?} differs from the folders in .claude/skills {expected:?}"
        ));
    }
    for d in &dirs {
        let text = read(&root.join(".claude/skills").join(d).join("SKILL.md"));
        match front_matter(&text) {
            None => errors.push(format!(".claude/skills/{d}/SKILL.md: no front matter")),
            Some(fm) => {
                let get = |k: &str| fm.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
                if get("name") != Some(d.as_str()) {
                    errors.push(format!(".claude/skills/{d}/SKILL.md: `name` must be `{d}`"));
                }
                if get("description").is_none_or(str::is_empty) {
                    errors.push(format!(
                        ".claude/skills/{d}/SKILL.md: missing `description`"
                    ));
                }
            }
        }
    }

    // Workflows.
    let mut files = Vec::new();
    files_under(&root.join(".github/workflows"), &mut files);
    for f in files {
        let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
        let text = read(&f);
        let code: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect();
        let jobs = code
            .iter()
            .filter(|l| l.trim_start().starts_with("runs-on:"))
            .count();
        let timeouts = code
            .iter()
            .filter(|l| l.trim_start().starts_with("timeout-minutes:"))
            .count();
        if jobs != timeouts {
            errors.push(format!(
                "{rel}: {jobs} job(s) but {timeouts} timeout-minutes"
            ));
        }
        for l in &code {
            if l.contains("pull_request_target") {
                errors.push(format!("{rel}: pull_request_target is not allowed"));
            }
            if l.contains("@master") || l.contains("@main") {
                errors.push(format!(
                    "{rel}: action reference on a moving branch: {}",
                    l.trim()
                ));
            }
        }
    }

    // Scratch files and blanket lint allowances.
    let mut all = Vec::new();
    files_under(&root.join("crates"), &mut all);
    files_under(&root.join("xtask"), &mut all);
    for f in all {
        let name = f
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
        if name.contains("scratch") || name.contains("probe") && !name.contains("asm_probe") {
            errors.push(format!("{rel}: scratch or probe file committed"));
        }
        if f.extension().is_some_and(|e| e == "rs") && !rel.ends_with("docs_check.rs") {
            let text = read(&f);
            if text.contains("clippy::all,")
                || text.contains("clippy::restriction") && text.contains("allow(")
            {
                errors.push(format!("{rel}: blanket lint allowance"));
            }
        }
    }
    for f in [
        "README.md",
        "SECURITY.md",
        "CITATION.cff",
        "MAINTAINERS.md",
        "Cargo.toml",
    ] {
        let text = read(&root.join(f));
        if text.contains("TODO(owner)") || text.contains("TODO-owner") {
            errors.push(format!("{f}: unresolved owner placeholder"));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_inline_code_and_invocations() {
        let t = "Run `cargo xtask ci fmt clippy` or\n```console\ncargo xtask validate-data # why\n```\n";
        assert_eq!(inline_code(t), vec!["cargo xtask ci fmt clippy"]);
        let inv = xtask_invocations(t);
        assert!(inv.contains(&vec!["ci".to_string(), "fmt".into(), "clippy".into()]));
        assert!(inv.contains(&vec!["validate-data".to_string()]));
    }

    #[test]
    fn parses_front_matter() {
        let fm = front_matter("---\nname: x\ndescription: Use when y\n---\nbody").unwrap();
        assert_eq!(fm[0], ("name".into(), "x".into()));
        assert!(front_matter("no front matter").is_none());
    }

    #[test]
    fn the_repository_passes() {
        let errors = run(&crate::repo_root());
        assert!(errors.is_empty(), "{errors:#?}");
    }
}
