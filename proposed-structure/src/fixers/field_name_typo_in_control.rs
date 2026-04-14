//! Fixer for field name casing typos in debian/control

use crate::{DebianFilesMut, DetectedIssue, FixerError};
use std::collections::HashSet;

const KNOWN_SOURCE_FIELDS: &[&str] = &[
    "Source",
    "Maintainer",
    "Uploaders",
    "Section",
    "Priority",
    "Build-Depends",
    "Build-Depends-Indep",
    "Build-Depends-Arch",
    "Build-Conflicts",
    "Build-Conflicts-Indep",
    "Build-Conflicts-Arch",
    "Standards-Version",
    "Homepage",
    "Vcs-Browser",
    "Vcs-Git",
    "Vcs-Svn",
    "Vcs-Bzr",
    "Vcs-Hg",
    "Vcs-Cvs",
    "Vcs-Darcs",
    "Vcs-Arch",
    "Vcs-Mtn",
    "Testsuite",
    "Testsuite-Triggers",
    "Rules-Requires-Root",
    "Origin",
    "Bugs",
    "X-Python-Version",
    "X-Python3-Version",
    "Xs-Go-Import-Path",
];

const KNOWN_BINARY_FIELDS: &[&str] = &[
    "Package",
    "Architecture",
    "Section",
    "Priority",
    "Essential",
    "Depends",
    "Pre-Depends",
    "Recommends",
    "Suggests",
    "Enhances",
    "Breaks",
    "Conflicts",
    "Provides",
    "Replaces",
    "Built-Using",
    "Static-Built-Using",
    "Multi-Arch",
    "Description",
    "Homepage",
    "Package-Type",
    "Build-Profiles",
    "Protected",
];

fn get_valid_fields() -> HashSet<&'static str> {
    let mut valid = HashSet::new();
    valid.extend(KNOWN_SOURCE_FIELDS.iter().copied());
    valid.extend(KNOWN_BINARY_FIELDS.iter().copied());
    valid
}

fn find_case_mismatch<'a>(field: &str, valid_fields: &HashSet<&'a str>) -> Option<&'a str> {
    if valid_fields.contains(field) {
        return None;
    }

    let field_lower = field.to_lowercase();
    valid_fields
        .iter()
        .find(|valid_field| valid_field.to_lowercase() == field_lower)
        .map(|v| &**v)
}

fn run(issues: &[DetectedIssue], files: &mut DebianFilesMut) -> Result<usize, FixerError> {
    let Some(control) = files.control.as_mut() else {
        return Ok(0);
    };
    debug_assert!(
        issues.iter().all(|issue| issue.tag == "cute-field"),
        "invalid input"
    );

    let editor = &mut control.editor;
    let valid_fields = get_valid_fields();
    let mut fixed = 0usize;

    if let Some(mut source) = editor.source() {
        let paragraph = source.as_mut_deb822();
        let entries: Vec<_> = paragraph
            .entries()
            .filter_map(|entry| entry.key().map(|key| (key, entry.line() + 1)))
            .collect();

        for (key, line) in entries {
            let Some(correct_field) = find_case_mismatch(&key, &valid_fields) else {
                continue;
            };

            let is_targeted = issues.iter().any(|issue| {
                issue.tag == "cute-field"
                    && issue.package.is_none()
                    && issue
                        .field
                        .as_deref()
                        .is_some_and(|f| f.eq_ignore_ascii_case(&key))
                    && issue.line.is_none_or(|l| l == line)
            });

            if is_targeted && paragraph.rename(&key, correct_field) {
                fixed += 1;
            }
        }
    }

    for mut binary in editor.binaries() {
        let package_name = binary.name();
        let paragraph = binary.as_mut_deb822();
        let entries: Vec<_> = paragraph
            .entries()
            .filter_map(|entry| entry.key().map(|key| (key, entry.line() + 1)))
            .collect();

        for (key, line) in entries {
            let Some(correct_field) = find_case_mismatch(&key, &valid_fields) else {
                continue;
            };

            let is_targeted = issues.iter().any(|issue| {
                issue.tag == "cute-field"
                    && issue.package.as_deref() == package_name.as_deref()
                    && issue
                        .field
                        .as_deref()
                        .is_some_and(|f| f.eq_ignore_ascii_case(&key))
                    && issue.line.is_none_or(|l| l == line)
            });

            if is_targeted && paragraph.rename(&key, correct_field) {
                fixed += 1;
            }
        }
    }

    Ok(fixed)
}

declare_fixer! {
    name: "field-name-typo-in-control",
    tag: "cute-field",
    description: "Normalizes field names to canonical casing in debian/control.",
    apply: run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_debian_files_mut, Fixer, PackageType};
    use std::fs;
    use tempfile::TempDir;

    fn setup_control_file(content: &str) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let debian_dir = temp_dir.path().join("debian");
        fs::create_dir_all(&debian_dir).unwrap();
        fs::write(debian_dir.join("control"), content).unwrap();
        temp_dir
    }

    #[test]
    fn test_fix_homepage_case() {
        let content = "Source: test\nHomePage: https://example.com\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "cute-field".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(2),
            field: Some("HomePage".to_string()),
            detector_name: "test-detector",
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(updated.contains("Homepage: https://example.com"));
        assert!(!updated.contains("HomePage:"));
    }

    #[test]
    fn test_fix_targets_specific_line_only() {
        let content =
            "Source: test\nHomePage: https://first.example\nHomePage: https://second.example\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "cute-field".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(2),
            field: Some("HomePage".to_string()),
            detector_name: "test-detector",
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(updated.contains("Homepage: https://first.example"));
        assert!(updated.contains("HomePage: https://second.example"));
    }
}
