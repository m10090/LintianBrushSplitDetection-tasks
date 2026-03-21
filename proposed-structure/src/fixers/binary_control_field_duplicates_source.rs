//! Fixer for duplicated source fields in binary paragraphs

use crate::{DebianFilesMut, DetectedIssue, FixerError};
use std::collections::HashMap;

fn run(issues: &[DetectedIssue], files: &mut DebianFilesMut) -> Result<usize, FixerError> {
    let Some(control) = files.control.as_mut() else {
        return Ok(0);
    };

    let editor = &mut control.editor;
    let Some(source) = editor.source() else {
        return Ok(0);
    };

    let source_fields: HashMap<String, String> = source.as_deb822().items().collect();
    let mut fixed = 0usize;

    for mut binary in editor.binaries() {
        let package_name = binary.name().unwrap_or_else(|| "unknown".to_string());
        let paragraph = binary.as_mut_deb822();
        let entries: Vec<_> = paragraph.entries().collect();
        let mut remove_keys: Vec<String> = Vec::new();

        for entry in entries {
            if let Some(key) = entry.key() {
                let value = entry.value();
                let line = entry.line() + 1;
                let matches_issue = issues.iter().any(|issue| {
                    issue.tag == "installable-field-mirrors-source"
                        && issue.package.as_deref() == Some(package_name.as_str())
                        && issue
                            .field
                            .as_deref()
                            .is_some_and(|f| f.eq_ignore_ascii_case(&key))
                        && issue.line.is_none_or(|l| l == line)
                });

                if matches_issue
                    && source_fields
                        .get(&key)
                        .is_some_and(|source_value| source_value == &value)
                {
                    remove_keys.push(key);
                    fixed += 1;
                }
            }
        }

        for key in remove_keys {
            paragraph.remove(&key);
        }
    }

    Ok(fixed)
}

declare_fixer! {
    name: "binary-control-field-duplicates-source",
    tag: "installable-field-mirrors-source",
    description: "Removes binary fields that duplicate values from source paragraph.",
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
    fn test_fix_duplicate_field() {
        let content = "Source: test\nPriority: optional\n\nPackage: test\nPriority: optional\nDescription: d\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "installable-field-mirrors-source".to_string(),
            description: "".to_string(),
            package: Some("test".to_string()),
            package_type: PackageType::Binary,
            line: Some(5),
            field: Some("Priority".to_string()),
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(!updated.contains("Package: test\nPriority: optional"));
    }
}
