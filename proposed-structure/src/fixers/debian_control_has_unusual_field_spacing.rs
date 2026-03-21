//! Fixer for unusual field spacing in debian/control

use crate::{DebianFilesMut, DetectedIssue, FixerError};

fn run(issues: &[DetectedIssue], files: &mut DebianFilesMut) -> Result<usize, FixerError> {
    let Some(control) = files.control.as_mut() else {
        return Ok(0);
    };

    let editor = &mut control.editor;
    let mut fixed = 0usize;

    if let Some(mut source) = editor.source() {
        let entries: Vec<_> = source.as_mut_deb822().entries().collect();
        for mut entry in entries {
            if let Some(key) = entry.key() {
                let line = entry.line() + 1;
                let should_fix = issues.iter().any(|issue| {
                    issue.tag == "debian-control-has-unusual-field-spacing"
                        && issue.package.is_none()
                        && issue
                            .field
                            .as_deref()
                            .is_some_and(|f| f.eq_ignore_ascii_case(&key))
                        && issue.line.is_none_or(|l| l == line)
                });

                if should_fix && entry.normalize_field_spacing() {
                    fixed += 1;
                }
            }
        }
    }

    for mut binary in editor.binaries() {
        let package_name = binary.name();
        let entries: Vec<_> = binary.as_mut_deb822().entries().collect();
        for mut entry in entries {
            if let Some(key) = entry.key() {
                let line = entry.line() + 1;
                let should_fix = issues.iter().any(|issue| {
                    issue.tag == "debian-control-has-unusual-field-spacing"
                        && issue.package.as_deref() == package_name.as_deref()
                        && issue
                            .field
                            .as_deref()
                            .is_some_and(|f| f.eq_ignore_ascii_case(&key))
                        && issue.line.is_none_or(|l| l == line)
                });

                if should_fix && entry.normalize_field_spacing() {
                    fixed += 1;
                }
            }
        }
    }

    Ok(fixed)
}

declare_fixer! {
    name: "debian-control-has-unusual-field-spacing",
    tag: "debian-control-has-unusual-field-spacing",
    description: "Normalizes spacing after field separators in debian/control.",
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
    fn test_fix_unusual_spacing() {
        let content = "Source: test\nBuild-Depends:  debhelper\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "debian-control-has-unusual-field-spacing".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(2),
            field: Some("Build-Depends".to_string()),
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(updated.contains("Build-Depends: debhelper"));
        assert!(!updated.contains("Build-Depends:  debhelper"));
    }
}
