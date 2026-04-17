//! Detector for field name typos (case mismatches) in debian/control
//!
//! Detects fields where the casing doesn't match the canonical Debian field names,
//! e.g., "HomePage" instead of "Homepage".

use crate::{DebianFiles, DetectedIssue, DetectorError, detectors::utils::get_package_type};
use std::collections::HashSet;

/// Known valid source field names in debian/control
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

/// Known valid binary field names in debian/control
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
    // Skip if it's already valid
    if valid_fields.contains(field) {
        return None;
    }

    // Look for case-insensitive match
    let field_lower = field.to_lowercase();
    valid_fields
        .iter()
        .find(|&&valid_field| valid_field.to_lowercase() == field_lower)
        .copied()
        .map(|v| v as _)
}

fn run(files: &DebianFiles) -> Result<Vec<DetectedIssue>, DetectorError> {
    let Some(control) = &files.control else {
        return Ok(vec![]);
    };

    let mut issues = Vec::new();
    let valid_fields = get_valid_fields();

    for paragraph in control.content.paragraphs() {
        let package_name = paragraph.get("Package");
        let package_type = get_package_type(&paragraph);

        for entry in paragraph.entries() {
            if let Some(key) = entry.key()
                && let Some(correct_field) = find_case_mismatch(&key, &valid_fields)
            {
                let line_number = entry.line() + 1;

                let description = format!(
                    "Field '{}' has incorrect casing, should be '{}' [{}:{}]",
                    key,
                    correct_field,
                    control.path.display(),
                    line_number
                );

                issues.push(DetectedIssue {
                    tag: "cute-field".to_string(),
                    description,
                    package: package_name.clone(),
                    package_type: package_type.clone(),
                    line: Some(line_number),
                    field: Some(key.to_string()),
                    action: Some(crate::Action::UpdateKey {
                        package: package_name.clone(),
                        package_type: package_type.clone(),
                        old_key: key,
                        new_key: correct_field.to_string(),
                    }),
                });
            }
        }
    }

    Ok(issues)
}

declare_detector! {
    name: "field-name-typo-in-control",
    tags: ["cute-field"],
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
    fn test_detect_homepage_case() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>
HomePage: https://example.com

Package: test-package
Architecture: any
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "cute-field");
        assert_eq!(issues[0].field, Some("HomePage".to_string()));
        assert!(issues[0].description.contains("Homepage"));
    }

    #[test]
    fn test_detect_maintainer_lowercase() {
        let content = r#"Source: test-package
maintainer: Test <test@example.com>

Package: test-package
Architecture: any
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].field, Some("maintainer".to_string()));
        assert!(issues[0].description.contains("Maintainer"));
    }

    #[test]
    fn test_detect_architecture_case_in_binary() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>

Package: test-package
architecture: any
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].field, Some("architecture".to_string()));
        assert_eq!(issues[0].package_type, PackageType::Binary);
    }

    #[test]
    fn test_multiple_typos() {
        let content = r#"Source: test-package
HomePage: https://example.com
maintainer: Test <test@example.com>

Package: test-package
architecture: any
Description: Test
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 3);
    }

    #[test]
    fn test_no_typos() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>
Homepage: https://example.com

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
    fn test_unknown_field_not_flagged() {
        // Unknown fields that don't match any known field shouldn't be flagged
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>
X-Custom-Field: value

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
