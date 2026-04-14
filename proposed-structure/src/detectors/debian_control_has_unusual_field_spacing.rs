//! Detector for unusual field spacing in debian/control
//!
//! Detects fields that have unusual spacing after the colon, such as
//! double spaces, tabs, or no space at all.
//!
//! Uses the deb822-lossless `normalize_field_spacing()` method to detect
//! fields that would need spacing normalization.

use crate::{DebianFiles, DetectedIssue, DetectorError, detectors::utils::get_package_type};

fn run(files: &DebianFiles) -> Result<Vec<DetectedIssue>, DetectorError> {
    let Some(control) = &files.control else {
        return Ok(vec![]);
    };

    let mut issues = Vec::new();

    for paragraph in control.content.paragraphs() {
        let package_name = paragraph.get("Package");
        let package_type = get_package_type(&paragraph);

        let entries: Vec<_> = paragraph.entries().collect();

        // Check each entry for unusual spacing
        for mut entry in entries {
            if let Some(key) = entry.key() {
                let line_number = entry.line() + 1;

                // normalize_field_spacing returns true if it would make changes
                // (i.e., if there's unusual spacing to fix)
                if entry.normalize_field_spacing() {
                    let description = format!(
                        "Field '{}' has unusual spacing [{}:{}]",
                        key,
                        control.path.display(),
                        line_number
                    );

                    issues.push(DetectedIssue {
                        tag: "debian-control-has-unusual-field-spacing".to_string(),
                        description,
                        package: package_name.clone(),
                        package_type: package_type.clone(),
                        line: Some(line_number),
                        field: Some(key.to_string()),
                    });
                }
            }
        }
    }

    Ok(issues)
}

declare_detector! {
    name: "debian-control-has-unusual-field-spacing",
    tags: ["debian-control-has-unusual-field-spacing"],
    detect: run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackageType;
    use crate::{Detector, load_debian_files};
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
    fn test_detect_double_space() {
        let content = "Source: test-package\nRecommends:  ${cdbs:Recommends}\n";
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "debian-control-has-unusual-field-spacing");
        assert_eq!(issues[0].field, Some("Recommends".to_string()));
    }

    #[test]
    fn test_detect_tab_after_colon() {
        let content = "Source: test-package\nBuild-Depends:\tpython3\n";
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].field, Some("Build-Depends".to_string()));
    }

    #[test]
    fn test_no_unusual_spacing() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>
Build-Depends: debhelper

Package: test-package
Architecture: any
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert!(issues.is_empty());
    }

    #[test]
    fn test_continuation_lines_ignored() {
        // Continuation lines with multiple spaces are normal
        let content = r#"Source: test-package
Build-Depends: debhelper,
  python3
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert!(issues.is_empty());
    }

    #[test]
    fn test_multiple_spacing_issues() {
        // Note: Tab-only spacing (e.g., "Field:\tvalue") is not recognized by
        // deb822-lossless entries() iterator. Use double spaces instead.
        let content = "Source: test-package\nBuild-Depends:  foo\nRecommends:  bar\n";
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn test_binary_package_spacing() {
        let content = r#"Source: test-package

Package: test-package
Architecture:  any
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].package_type, PackageType::Binary);
    }

    #[test]
    fn test_empty_field_not_flagged() {
        // Empty fields are handled by a different detector
        let content = "Source: test-package\nDepends:\n";
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert!(issues.is_empty());
    }

    #[test]
    fn test_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        // No control file means no issues (not an error)
        let issues = detector.detect(&files).unwrap();
        assert!(issues.is_empty());
    }
}
