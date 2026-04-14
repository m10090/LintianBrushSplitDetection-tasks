//! Detectors for debian/control files
//!
//! This module provides detectors that identify issues in debian/control files
//! without applying any fixes. Each detector returns a list of detected issues.

#[macro_use]
mod macros;

pub mod detectors;

use deb822_lossless::Deb822;
use debian_analyzer::control::TemplatedControlEditor;
use debian_analyzer::editor::EditorError;
use lazy_static::lazy_static;
use std::collections::HashMap;
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
    /// Detector's name that found the issue
    pub detector_name: &'static str,
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
    fn detect(
        &self,
        files: &DebianFiles,
    ) -> Result<Vec<(DetectedIssue, Option<Box<dyn FnOnce()>>)>, DetectorError>;

    /// Get the name of this detector
    fn name(&self) -> &'static str;

    /// Get the lintian tags this detector looks for
    fn tags(&self) -> &'static [&'static str];
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

lazy_static! {
    static ref DETECTORS: HashMap<&'static str, &'static dyn Detector> = {
        let mut map = HashMap::new();
        for reg in inventory::iter::<DetectorRegistration> {
            let detectors = (reg.create)();
            let detectors: &'static dyn Detector = Box::leak(detectors);
            debug_assert!(!map.contains_key(reg.name), "Duplicate detector name found");
            map.insert(reg.name, detectors);
        }
        map
    };
}

/// Get all registered detectors
pub fn get_detectors() -> Vec<&'static dyn Detector> {
    let mut dets: Vec<_> = DETECTORS.values().copied().collect();
    dets.sort_unstable_by_key(|d| d.name());
    dets
}

/// Run all registered detectors and collect all issues
pub fn detect_all(
    base_path: &Path,
) -> Result<Vec<(DetectedIssue, Option<Box<dyn FnOnce()>>)>, DetectorError> {
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

// #[cfg(test)]
// mod tests {
// comment out for refactoring
// }
