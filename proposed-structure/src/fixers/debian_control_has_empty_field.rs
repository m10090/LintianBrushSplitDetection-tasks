//! Fixer for empty fields in debian/control

use crate::{DebianFilesMut, DetectedIssue, FixerError, PackageType};

fn issue_matches(
    issue: &DetectedIssue,
    package_type: &PackageType,
    package_name: Option<&str>,
    field: &str,
    line: usize,
) -> bool {
    if issue.tag != "debian-control-has-empty-field" {
        return false;
    }

    if issue.package_type != *package_type {
        return false;
    }

    if issue.package.as_deref() != package_name {
        return false;
    }

    if !issue
        .field
        .as_deref()
        .is_some_and(|f| f.eq_ignore_ascii_case(field))
    {
        return false;
    }

    issue.line.is_none_or(|l| l == line)
}

fn run(issues: &[DetectedIssue], files: &mut DebianFilesMut) -> Result<usize, FixerError> {
    let Some(control) = files.control.as_mut() else {
        return Ok(0);
    };

    let editor = &mut control.editor;
    let mut fixed = 0usize;

    if let Some(mut source) = editor.source() {
        let paragraph = source.as_mut_deb822();
        let entries: Vec<_> = paragraph.entries().collect();
        let mut keys_to_remove: Vec<String> = Vec::new();

        for entry in entries {
            if let Some(key) = entry.key() {
                let line = entry.line() + 1;
                if entry.value().trim().is_empty()
                    && issues
                        .iter()
                        .any(|issue| issue_matches(issue, &PackageType::Source, None, &key, line))
                {
                    keys_to_remove.push(key);
                    fixed += 1;
                }
            }
        }

        for key in keys_to_remove {
            paragraph.remove(&key);
        }
    }

    for mut binary in editor.binaries() {
        let package_name = binary.name();
        let paragraph = binary.as_mut_deb822();

        let entries: Vec<_> = paragraph.entries().collect();
        let mut keys_to_remove: Vec<String> = Vec::new();

        for entry in entries {
            if let Some(key) = entry.key() {
                let line = entry.line() + 1;
                if entry.value().trim().is_empty()
                    && issues.iter().any(|issue| {
                        issue_matches(
                            issue,
                            &PackageType::Binary,
                            package_name.as_deref(),
                            &key,
                            line,
                        )
                    })
                {
                    keys_to_remove.push(key);
                    fixed += 1;
                }
            }
        }

        for key in keys_to_remove {
            paragraph.remove(&key);
        }
    }

    Ok(fixed)
}

declare_fixer! {
    name: "debian-control-has-empty-field",
    tag: "debian-control-has-empty-field",
    description: "Removes empty fields from debian/control paragraphs.",
    apply: run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_debian_files_mut, Fixer};
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
    fn test_fix_empty_source_field() {
        let content = "Source: test\nMaintainer:\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "debian-control-has-empty-field".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(2),
            field: Some("Maintainer".to_string()),
            detector_name: "test-detector",
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(!updated.contains("Maintainer:"));
    }

    #[test]
    fn test_fix_ignores_non_matching_issues() {
        let content = "Source: test\nMaintainer:\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "some-other-tag".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(2),
            field: Some("Maintainer".to_string()),
            detector_name: "test-detector",
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();

        assert_eq!(fixed, 0);
    }
}
