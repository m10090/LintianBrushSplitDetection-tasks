//! Detector for ancient X-Python-Version / X-Python3-Version fields
//!
//! Detects `X-Python-Version` and `X-Python3-Version` fields in the source
//! paragraph that use a `>= MAJOR.MINOR` range and are considered ancient
//! according to lintian's thresholds (or builtin fallbacks).

use crate::{DebianFiles, DetectedIssue, DetectorError, PackageType};
use std::fs;

use super::utils::get_source_paragraph;

// Fallback thresholds when the system lintian file is not present.
const FALLBACK_OLD_PY2: (u8, u8) = (2, 7);
const FALLBACK_OLD_PY3: (u8, u8) = (3, 7);

fn parse_version(value: &str) -> Option<(u8, u8)> {
    // Expect values like ">= 3.2" (we are tolerant about whitespace)
    let s = value.trim();
    if !s.starts_with(">=") {
        return None;
    }
    let ver_part = s[2..].trim();
    if ver_part.is_empty() {
        return None;
    }

    let mut parts = ver_part.splitn(2, '.');
    let major = parts.next()?.trim().parse::<u8>().ok()?;
    let minor = parts
        .next()
        .and_then(|m| m.trim().parse::<u8>().ok())
        .unwrap_or(0);
    Some((major, minor))
}

fn find_version_in_line(line: &str) -> Option<(u8, u8)> {
    // Look for tokens that look like MAJOR.MINOR
    for token in line.split_whitespace() {
        let token = token.trim_matches(|c: char| c == ':' || c == ',' || c == ';');
        if token.chars().all(|c| c.is_ascii_digit() || c == '.') && token.contains('.') {
            let mut parts = token.splitn(2, '.');
            if let (Some(maj), Some(min)) = (parts.next(), parts.next()) {
                if let (Ok(maj), Ok(min)) = (maj.parse::<u8>(), min.parse::<u8>()) {
                    return Some((maj, min));
                }
            }
        }
    }
    None
}

fn load_thresholds() -> ((u8, u8), (u8, u8)) {
    let path = "/usr/share/lintian/data/python/versions";

    let mut py2 = None;
    let mut py3 = None;

    if let Ok(content) = fs::read_to_string(path) {
        for raw_line in content.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            if line.contains("old-python3") {
                if let Some(v) = find_version_in_line(line) {
                    py3 = Some(v);
                }
            }

            if line.contains("old-python2") || line.contains("old-python") {
                if let Some(v) = find_version_in_line(line) {
                    py2 = Some(v);
                }
            }
        }
    }

    (py2.unwrap_or(FALLBACK_OLD_PY2), py3.unwrap_or(FALLBACK_OLD_PY3))
}

fn run(files: &DebianFiles) -> Result<Vec<DetectedIssue>, DetectorError> {
    let Some(control) = &files.control else {
        return Ok(vec![]);
    };

    let mut issues = Vec::new();

    let source = match get_source_paragraph(control.content) {
        None => return Ok(vec![]),
        Some(p) => p,
    };

    let (py2_thresh, py3_thresh) = load_thresholds();

    for entry in source.entries() {
        if let Some(key) = entry.key()
            && (key == "X-Python-Version" || key == "X-Python3-Version")
            && let value = entry.value()
            && let Some(parsed) = parse_version(&value)
        {
            let is_ancient = if key == "X-Python-Version" {
                parsed <= py2_thresh
            } else {
                parsed <= py3_thresh
            };

            if is_ancient {
                let line_number = entry.line() + 1;

                let description = format!(
                    "Field '{}' in source paragraph specifies ancient python version '{}' [{}:{}]",
                    key,
                    value.trim(),
                    control.path.display(),
                    line_number
                );

                issues.push(DetectedIssue {
                    tag: "ancient-python-version-field".to_string(),
                    description,
                    package: None,
                    package_type: PackageType::Source,
                    line: Some(line_number),
                    field: Some(key.to_string()),
                    action: Some(crate::Action::DeleteKey {
                        package: None,
                        package_type: PackageType::Source,
                        key,
                    }),
                });
            }
        }
    }

    Ok(issues)
}

declare_detector! {
    name: "ancient-python-version-field",
    tags: ["ancient-python-version-field"],
    detect: run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Detector, load_debian_files};
    use std::fs;
    use tempfile::TempDir;
    use crate::PackageType;

    fn setup_control_file(content: &str) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let debian_dir = temp_dir.path().join("debian");
        fs::create_dir_all(&debian_dir).unwrap();
        fs::write(debian_dir.join("control"), content).unwrap();
        temp_dir
    }

    #[test]
    fn test_detect_ancient_x_python_version() {
        let content = r#"Source: test-package
X-Python-Version: >= 2.5
Maintainer: Test <test@example.com>

Package: test-package
Architecture: any
Description: Test package
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "ancient-python-version-field");
        assert_eq!(issues[0].field, Some("X-Python-Version".to_string()));
        assert_eq!(issues[0].package_type, PackageType::Source);
        assert!(issues[0].package.is_none());
    }

    #[test]
    fn test_detect_ancient_x_python3_version() {
        let content = r#"Source: test-package
X-Python3-Version: >= 3.2
Maintainer: Test <test@example.com>

Package: test-package
Architecture: any
Description: Test package
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "ancient-python-version-field");
        assert_eq!(issues[0].field, Some("X-Python3-Version".to_string()));
    }

    #[test]
    fn test_ignore_recent_x_python3_version() {
        let content = r#"Source: test-package
X-Python3-Version: >= 3.8
Maintainer: Test <test@example.com>

Package: test-package
Architecture: any
Description: Test package
"#;
        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();

        assert!(issues.is_empty());
    }

    #[test]
    fn test_no_control_file() {
        let temp_dir = TempDir::new().unwrap();
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();
        assert!(issues.is_empty());
    }
}
