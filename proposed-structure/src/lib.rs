//! Detectors for debian/control files
//!
//! This module provides detectors that identify issues in debian/control files
//! without applying any fixes. Each detector returns a list of detected issues.

#[macro_use]
mod macros;

mod apply_action;
pub mod detectors;
pub use apply_action::apply_action;

use deb822_lossless::Deb822;
use debian_control::lossless::relations::Relations;
use debian_control::lossy::Relation;
use debian_analyzer::control::TemplatedControlEditor;
use debian_analyzer::editor::EditorError;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use thiserror::Error;

/// Wrapper for a parsed debian/control file
#[derive(Debug, Clone)]
pub struct ControlFile<'a> {
    /// The parsed deb822 content
    pub content: &'a Deb822,
    /// Path to the control file
    pub path: &'a Path,
}

/// Collection of all Debian source files (pre-parsed)
///
/// Each field is optional - detectors check for the files they need.
#[derive(Debug, Default)]
pub struct DebianFiles<'a> {
    /// The debian/control file (if present and parsed)
    pub control: Option<ControlFile<'a>>,
    // Future fields:
    // pub changelog: Option<ChangelogFile<'a>>,
    // pub rules: Option<RulesFile<'a>>,
    // pub copyright: Option<CopyrightFile<'a>>,
}

/// Owns the parsed data and provides references via `DebianFiles`
///
/// This struct owns the parsed content while `DebianFiles` holds references to it.
#[derive(Debug)]
pub struct LoadedFiles {
    control_path: PathBuf, // if needed
    control_content: Option<Deb822>,
}

/// Mutable wrapper for a parsed debian/control file used by [crate::apply_action::apply_action]
pub struct ControlFileMut {
    /// Mutable editor for debian/control (supports templates)
    pub editor: TemplatedControlEditor,
    /// Path to the control file
    pub path: PathBuf,
}

/// Mutable collection of Debian files used by fixers
#[derive(Default)]
pub struct DebianFilesMut {
    /// Mutable debian/control file content
    pub control: Option<ControlFileMut>,
}

impl DebianFilesMut {
    /// Persist modified files back to disk
    pub fn write_back(&self) -> Result<(), FixerError> {
        if let Some(control) = &self.control {
            control.editor.commit()?;
        }
        Ok(())
    }
}

impl LoadedFiles {
    /// Create a `DebianFiles` view with references to the loaded content
    pub fn as_ref(&self) -> DebianFiles<'_> {
        DebianFiles {
            control: self.control_content.as_ref().map(|content| ControlFile {
                content,
                path: &self.control_path,
            }),
        }
    }
}

/// Load and parse Debian source files from the given base path
pub fn load_debian_files(base_path: &Path) -> Result<LoadedFiles, DetectorError> {
    let control_path = base_path.join("debian/control");

    let control_content = if control_path.exists() {
        let content_str = fs::read_to_string(&control_path)?;
        let parsed =
            Deb822::from_str(&content_str).map_err(|e| DetectorError::ParseError(e.to_string()))?;
        Some(parsed)
    } else {
        None
    };

    Ok(LoadedFiles {
        control_path,
        control_content,
    })
}

/// Load and parse mutable Debian source files for fixers
pub fn load_debian_files_mut(base_path: &Path) -> Result<DebianFilesMut, FixerError> {
    let control_path = base_path.join("debian/control");

    let control = match TemplatedControlEditor::open(&control_path) {
        Ok(editor) => Some(ControlFileMut {
            editor,
            path: control_path,
        }),
        Err(EditorError::IoError(err)) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(FixerError::EditorError(err)),
    };

    Ok(DebianFilesMut { control })
}

/// Represents a detected issue in a debian/control file
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedIssue {
    /// The lintian tag associated with this issue
    pub tag: String,
    /// Human-readable description of the issue
    pub description: String,
    /// The package this issue relates to (None for source package)
    pub package: Option<String>,
    /// Whether this is a source or binary package issue
    pub package_type: PackageType,
    /// Line number where the issue was found (1-indexed)
    pub line: Option<usize>,
    /// The field name involved (if applicable)
    pub field: Option<String>,
    /// Action to fix the issue
    pub action: Option<Action>, // this could be an array in future
}

#[derive(Debug, Clone, PartialEq)]
enum Action {
    UpdateKey {
        package: Option<String>,
        package_type: PackageType,
        old_key: String,
        new_key: String,
    },
    DeleteKey {
        package: Option<String>,
        package_type: PackageType,
        key: String,
    },
    UpdateValue {
        package: Option<String>,
        package_type: PackageType,
        key: String,
        current_value: String,
        new_value: String,
    },
    /// Structured relation update: drop specific dependencies and ensure minimum
    /// versions for specified packages. This is preferable to replacing the
    /// entire field text because it describes semantic edits.
    UpdateRelation {
        package: Option<String>,
        package_type: PackageType,
        key: String,
        remove: Vec<Relation>,
        add: Vec<Relation>,
    },
    NormalizeFieldSpacing {
        package: Option<String>,
        package_type: PackageType,
        key: String,
    },
}

/// Type of package where the issue was found
#[derive(Debug, Clone, PartialEq)]
pub enum PackageType {
    Source,
    Binary,
}

/// Errors that can occur during detection
#[derive(Error, Debug)]
pub enum DetectorError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Failed to parse control file: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Errors that can occur during fixing
#[derive(Error, Debug)]
pub enum FixerError {
    #[error("Failed to parse control file: {0}")]
    ParseError(String),
    #[error("Control editor error: {0}")]
    EditorError(#[from] EditorError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Trait for all detectors
pub trait Detector: Send + Sync {
    /// Detect issues in the provided Debian files
    fn detect(&self, files: &DebianFiles) -> Result<Vec<DetectedIssue>, DetectorError>;

    /// Get the name of this detector
    fn name(&self) -> &'static str;

    /// Get the lintian tags this detector looks for
    fn tags(&self) -> &'static [&'static str];
}

/// Trait for all fixers
pub trait Fixer: Send + Sync {
    /// Name of the fixer
    fn name(&self) -> &'static str;

    /// The lintian tag this fixer handles
    fn tag(&self) -> &'static str;

    /// Human-readable fixer description
    fn description(&self) -> &'static str;

    /// Apply fixes for provided issues to mutable Debian files
    fn apply(
        &self,
        issues: &[DetectedIssue],
        files: &mut DebianFilesMut,
    ) -> Result<usize, FixerError>;
}

/// Registration information for a detector
pub struct DetectorRegistration {
    /// Name of the detector
    pub name: &'static str,
    /// Lintian tags this detector looks for
    pub lintian_tags: &'static [&'static str],
    /// Function to create an instance of the detector
    pub create: fn() -> Box<dyn Detector>,
}


inventory::collect!(DetectorRegistration);


/// Get all registered detectors
pub fn get_detectors() -> Vec<Box<dyn Detector>> {
    inventory::iter::<DetectorRegistration>
        .into_iter()
        .map(|reg| (reg.create)())
        .collect()
}


/// Run all registered detectors and collect all issues
pub fn detect_all(base_path: &Path) -> Result<Vec<DetectedIssue>, DetectorError> {
    // TODO: add the topological sort
    let loaded = load_debian_files(base_path)?;
    let files = loaded.as_ref();

    let mut all_issues = Vec::new();
    for detector in get_detectors() {
        let issues = detector.detect(&files)?;
        all_issues.extend(issues);
    }

    Ok(all_issues)
}

/// Apply all registered fixers to the provided issues
pub fn fix_all(base_path: &Path, issues: &[DetectedIssue]) -> Result<usize, FixerError> {
    let mut files = load_debian_files_mut(base_path)?;

    // Apply actions emitted by detectors directly to the editor-backed files.
    // This uses the new Action enum carried by DetectedIssue.
    // First, apply any explicit actions produced by detectors.
    let fixed_count = apply_action(&mut files, issues);


    files.write_back()?;
    Ok(fixed_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn setup_control_file(content: &str) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let debian_dir = temp_dir.path().join("debian");
        fs::create_dir_all(&debian_dir).unwrap();
        fs::write(debian_dir.join("control"), content).unwrap();
        temp_dir
    }

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test/hello-debian")
    }

    fn setup_fixture_copy() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let debian_dir = temp_dir.path().join("debian");
        fs::create_dir_all(&debian_dir).unwrap();

        let fixture_control = fixture_path().join("debian/control");
        let content = fs::read_to_string(fixture_control).unwrap();
        fs::write(debian_dir.join("control"), content).unwrap();

        temp_dir
    }

    #[test]
    fn test_get_detectors_returns_registered() {
        let detectors = get_detectors();
        // We should have at least 4 detectors registered
        assert!(
            detectors.len() >= 4,
            "Expected at least 4 detectors, got {}",
            detectors.len()
        );

        // Check that specific detectors are registered
        let names: Vec<_> = detectors.iter().map(|d| d.name()).collect();
        assert!(names.contains(&"debian-control-has-empty-field"));
        assert!(names.contains(&"binary-control-field-duplicates-source"));
        assert!(names.contains(&"field-name-typo-in-control"));
        assert!(names.contains(&"debian-control-has-unusual-field-spacing"));
    }

    #[test]
    fn test_detect_all_finds_issues() {
        let content = r#"Source: test-package
Maintainer:
HomePage: https://example.com

Package: test-package
Architecture:  any
Description: Test
"#;
        let temp_dir = setup_control_file(content);

        let issues = detect_all(temp_dir.path()).unwrap();

        // Should find: empty Maintainer, HomePage case, double space in Architecture
        assert!(
            issues.len() >= 3,
            "Expected at least 3 issues, got {}",
            issues.len()
        );
    }

    #[test]
    fn test_detect_all_no_issues() {
        let content = r#"Source: test-package
Maintainer: Test <test@example.com>

Package: test-package
Architecture: any
Description: Test package
"#;
        let temp_dir = setup_control_file(content);

        let issues = detect_all(temp_dir.path()).unwrap();

        assert!(issues.is_empty(), "Expected no issues, got {:?}", issues);
    }

    #[test]
    fn test_fix_all_uses_only_matching_tag_issues() {
        let content = "Source: test-package\nMaintainer:\nHomePage: https://example.com\n\nPackage: test-package\nArchitecture: any\nDescription: Test\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "cute-field".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(3),
            field: Some("HomePage".to_string()),
            action: Some(Action::UpdateKey {
                package: None,
                package_type: PackageType::Source,
                old_key: "HomePage".to_string(),
                new_key: "Homepage".to_string(),
            }),
        }];

        let fixed = fix_all(temp_dir.path(), &issues).unwrap();
        assert_eq!(fixed, 1);

        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(updated.contains("Homepage: https://example.com"));
        assert!(updated.contains("Maintainer:"));
    }

    #[test]
    fn test_hello_debian_fixture_detects_all_supported_issues() {
        let fixture_control = fixture_path().join("debian/control");
        let fixture_before = fs::read_to_string(&fixture_control).unwrap();

        let temp_dir = setup_fixture_copy();
        let issues = detect_all(temp_dir.path()).unwrap();
        assert_eq!(issues.len(), 6, "expected 6 issues, got: {issues:?}");

        let mut by_tag: HashMap<&str, usize> = HashMap::new();
        for issue in &issues {
            *by_tag.entry(issue.tag.as_str()).or_insert(0) += 1;
        }

        assert_eq!(by_tag.get("debian-control-has-empty-field"), Some(&2));
        assert_eq!(by_tag.get("installable-field-mirrors-source"), Some(&1));
        assert_eq!(by_tag.get("cute-field"), Some(&1));
        assert_eq!(
            by_tag.get("debian-control-has-unusual-field-spacing"),
            Some(&2)
        );

        let fixture_after = fs::read_to_string(&fixture_control).unwrap();
        assert_eq!(fixture_before, fixture_after);
    }

    #[test]
    fn test_hello_debian_fixture_fixers_resolve_all_issues() {
        let fixture_control = fixture_path().join("debian/control");
        let fixture_before = fs::read_to_string(&fixture_control).unwrap();

        let temp_dir = setup_fixture_copy();
        let mut iterations = 0usize;

        loop {
            let issues = detect_all(temp_dir.path()).unwrap();
            if issues.is_empty() {
                break;
            }

            let fixed_count = fix_all(temp_dir.path(), &issues).unwrap();
            assert!(
                fixed_count > 0,
                "no progress while issues remain: {issues:?}"
            );

            iterations += 1;
            assert!(
                iterations <= 8,
                "fixers did not converge after {iterations} iterations"
            );
        }

        let fixture_after = fs::read_to_string(&fixture_control).unwrap();
        assert_eq!(fixture_before, fixture_after);
    }
}
