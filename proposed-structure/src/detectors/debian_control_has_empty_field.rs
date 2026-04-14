//! Detector for empty fields in debian/control
//!
//! Detects fields that have empty or whitespace-only values in both
//! source and binary package paragraphs.
const DETECTOR_NAME: &str = "debian-control-has-empty-field";

use super::utils::get_package_type;
use crate::{DebianFiles, DetectedIssue, DetectorError, PackageType};

fn run(files: &DebianFiles) -> Result<Vec<DetectedIssue>, DetectorError> {
    let Some(control) = &files.control else {
        return Ok(vec![]);
    };

    let mut issues = Vec::new();

    for paragraph in control.content.paragraphs() {
        let package_name = paragraph.get("Package");
        let package_type = get_package_type(&paragraph);

        for entry in paragraph.entries() {
            if let Some(key) = entry.key()
                && let value = entry.value()
                && value.trim().is_empty()
            {
                let line_number = entry.line() + 1;

                let description = format!(
                    "Empty field '{}' in {} package '{}' [{}:{}]",
                    key,
                    if package_type == PackageType::Source {
                        "source"
                    } else {
                        "binary"
                    },
                    package_name.as_deref().unwrap_or("unknown"),
                    control.path.display(),
                    line_number
                );

                issues.push(DetectedIssue {
                    tag: "debian-control-has-empty-field".to_string(),
                    description,
                    package: package_name.clone(),
                    package_type: package_type.clone(),
                    line: Some(line_number),
                    field: Some(key.to_string()),
                    detector_name: DETECTOR_NAME,
                });
            }
        }
    }

    Ok(issues)
}

declare_detector! {
    tags: ["debian-control-has-empty-field"],
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
    fn test_detect_empty_field_in_source() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>
Depends:

Package: test-package
Architecture: any
Description: Test package
 A test package
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "debian-control-has-empty-field");
        assert_eq!(issues[0].field, Some("Depends".to_string()));
        assert_eq!(issues[0].package_type, PackageType::Source);
    }

    #[test]
    fn test_detect_empty_field_in_binary() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>

Package: test-package
Architecture: any
Provides:
Description: Test package
 A test package
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "debian-control-has-empty-field");
        assert_eq!(issues[0].field, Some("Provides".to_string()));
        assert_eq!(issues[0].package_type, PackageType::Binary);
        assert_eq!(issues[0].package, Some("test-package".to_string()));
    }

    #[test]
    fn test_detect_whitespace_only_field() {
        let content = "Source: test-package\nBuild-Depends:   \t\n\nPackage: test-package\nArchitecture: any\nDescription: Test package\n";
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].field, Some("Build-Depends".to_string()));
    }

    #[test]
    fn test_no_empty_fields() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>
Build-Depends: debhelper

Package: test-package
Architecture: any
Depends: libc6
Description: Test package
 A test package
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert!(issues.is_empty());
    }

    #[test]
    fn test_multiple_empty_fields() {
        let content = r#"Source: test-package
Maintainer:
Build-Depends:

Package: test-package
Architecture: any
Provides:
Description: Test package
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 3);
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
