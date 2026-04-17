//! Detector for binary fields that duplicate source fields in debian/control
//!
//! Detects when a binary package paragraph contains a field with the same
//! name and value as the source paragraph, which is redundant.

use crate::{
    DebianFiles, DetectedIssue, DetectorError, PackageType,
    detectors::utils::{get_binary_paragraphs, get_source_paragraph},
};
use std::collections::HashMap;

fn run(files: &DebianFiles) -> Result<Vec<DetectedIssue>, DetectorError> {
    let Some(control) = &files.control else {
        return Ok(vec![]);
    };

    let mut issues = Vec::new();

    // Get source paragraph fields using items() for key-value pairs
    let source_fields: HashMap<String, String> = match get_source_paragraph(control.content) {
        None => {
            return Ok(vec![]);
        }
        Some(paragraph) => paragraph
            .keys()
            .map(|k| (k.to_string(), paragraph.get(&k).unwrap_or_default()))
            .collect(),
    };

    let binaries = get_binary_paragraphs(control.content);
    // Check binary paragraphs
    for binary in binaries {
        let package_name = binary.get("Package");
        // Use entries() to get line numbers directly
        for entry in binary.entries() {
            if let Some(key) = entry.key()
                && let Some(source_value) = source_fields.get(&key.to_string())
                && let value = entry.value()
                && source_value == &value
            {
                let line_number = entry.line() + 1; // 1-indexed

                issues.push(DetectedIssue {
                    tag: "installable-field-mirrors-source".to_string(),
                    description: format!(
                        "Field '{}' in binary package '{}' duplicates source paragraph value '{}'",
                        key,
                        package_name
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string()),
                        value.trim()
                    ),
                    package: package_name.clone(),
                    package_type: PackageType::Binary,
                    line: Some(line_number),
                    field: Some(key.to_string()),
                    action: Some(crate::Action::DeleteKey {
                        package: package_name.clone(),
                        package_type: PackageType::Binary,
                        key,
                    }),
                });
            }
        }
    }

    Ok(issues)
}

declare_detector! {
    name: "binary-control-field-duplicates-source",
    tags: ["installable-field-mirrors-source"],
    detect: run
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_detect_duplicate_priority() {
        let content = r#"Source: test-package
Section: net
Priority: optional

Package: test-package
Architecture: any
Section: vcs
Priority: optional
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "installable-field-mirrors-source");
        assert_eq!(issues[0].field, Some("Priority".to_string()));
        assert_eq!(issues[0].package, Some("test-package".to_string()));
    }

    #[test]
    fn test_detect_multiple_duplicates() {
        let content = r#"Source: test-package
Section: net
Priority: optional

Package: test-package
Architecture: any
Section: net
Priority: optional
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 2);
        let fields: Vec<_> = issues.iter().filter_map(|i| i.field.clone()).collect();
        assert!(fields.contains(&"Section".to_string()));
        assert!(fields.contains(&"Priority".to_string()));
    }

    #[test]
    fn test_no_duplicates_different_values() {
        let content = r#"Source: test-package
Section: net
Priority: optional

Package: test-package
Architecture: any
Section: vcs
Priority: extra
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
    fn test_no_duplicates_binary_only_fields() {
        let content = r#"Source: test-package
Section: net

Package: test-package
Architecture: any
Depends: libc6
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
    fn test_multiple_binary_packages() {
        let content = r#"Source: test-package
Section: net
Priority: optional

Package: test-package
Architecture: any
Priority: optional
Description: Test

Package: test-package-dev
Architecture: any
Priority: optional
Description: Test dev
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 2);
        let packages: Vec<_> = issues.iter().filter_map(|i| i.package.clone()).collect();
        assert!(packages.contains(&"test-package".to_string()));
        assert!(packages.contains(&"test-package-dev".to_string()));
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
