//! Detectors for debian/control files
//!
//! This module provides detectors that identify issues in debian/control files
//! without applying any fixes. Each detector returns a list of detected issues.

#[macro_use]
mod macros;

pub mod detectors;

use debian_analyzer::control::TemplatedControlEditor;
use debian_analyzer::editor::EditorError;
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Collection of all Debian source files (pre-parsed)
///
/// Each field is optional - detectors check for the files they need.
#[derive(Default)]
pub struct DebianFiles {
    /// The debian/control file (if present and parsed)
    pub control: Option<TemplatedControlEditor>,
    // Future fields:
    // pub changelog: Option<ChangelogFile<'a>>,
    // pub rules: Option<RulesFile<'a>>,
    // pub copyright: Option<CopyrightFile<'a>>,
}

impl DebianFiles {
    pub fn write_back(&mut self) -> Result<(), DetectorError> {
        if let Some(control) = self.control.as_ref() {
            control.commit()?;
        }
        Ok(())
    }
}

/// Load and parse Debian source files from the given base path
pub fn load_debian_files(base_path: &Path) -> Result<DebianFiles, DetectorError> {
    let control_path = base_path.join("debian/control");

    let control = if control_path.exists() {
        let parsed = TemplatedControlEditor::new(control_path, false)?;
        Some(parsed)
    } else {
        None
    };

    Ok(DebianFiles { control })
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
    #[error("IO error: {0}")]
    EditorError(#[from] EditorError),
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
    pub static ref DETECTORS: HashMap<&'static str, &'static dyn Detector> = {
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
pub fn detect_all(base_path: &Path) -> Result<Vec<DetectedIssue>, DetectorError> {
    // TODO: add the topological sort
    let files = load_debian_files(base_path)?;

    let mut all_issues = Vec::new();
    for detector in get_detectors() {
        let issues = detector.detect(&files)?.into_iter().map(|res| res.0);
        all_issues.extend(issues);
    }

    Ok(all_issues)
}
pub fn apply_all_fixers(
    path: &Path,
    issues: &[DetectedIssue],
    dry_run: bool,
) -> Result<usize, String> {
    apply_fixers(
        path,
        issues,
        DETECTORS
            .keys()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .as_ref(), // this is so bad I know
        dry_run,
    )
}

// pub fn apply_fixers(
//     path: PathBuf,
//     issues: &[DetectedIssue],
//     selected: &[&'static str],
//     dry_run: bool,
// ) -> Result<usize, DetectorError> {
//     let files = load_debian_files(&path)?;
//     // find detector to solve the issue with
//     let mut set: HashSet<&'static str> = HashSet::new();
//     let mut selected_set: HashSet<&'static str> = HashSet::new();
//     for selected_fixer in selected {
//         selected_set.insert(selected_fixer);
//     }
//     for DetectedIssue { detector_name, .. } in issues {
//         set.insert(detector_name);
//     }
//     let mut fixed = 0;
//     for detector_name in selected_set.difference(&set) {
//         let Some(detectors) = DETECTORS.get(detector_name) else {
//             unreachable!()
//         };
//         fixed += detectors
//             .detect(&files)?
//             .into_iter()
//             .fold(0, |acc, (_, fix)| {
//                 if let Some(fix) = fix {
//                     // apply
//                     fix();
//                     acc + 1
//                 } else {
//                     acc
//                 }
//             });
//     }
//     if !dry_run {
//         eprintln!("this mode isn't suppprted now");
//     }
//     Ok(fixed)
// }
pub fn apply_detector_fix(
    detector_name: &str,
    files: &DebianFiles,
) -> Result<usize, DetectorError> {
    let Some(detector) = DETECTORS.get(detector_name) else {
        return Ok(0);
    };
    let mut fixed = 0;
    for (_, fix) in detector.detect(files)? {
        let Some(fix) = fix else {
            continue;
        };
        fix();
        fixed += 1;
    }
    Ok(fixed)
}
pub fn apply_fixers(
    base_path: &std::path::Path,
    issues: &[DetectedIssue],
    fixer_names: &[String],
    dry_run: bool,
) -> Result<usize, String> {
    let mut files = load_debian_files(base_path).map_err(|e| e.to_string())?;

    let mut issues_by_tag: HashMap<&str, Vec<DetectedIssue>> = HashMap::new();
    for issue in issues {
        issues_by_tag
            .entry(issue.tag.as_str())
            .or_default()
            .push(issue.clone());
    }

    let mut fixed_total = 0usize;
    for name in fixer_names {
        fixed_total += apply_detector_fix(name, &files).map_err(|e| e.to_string())?;
    }

    if !dry_run {
        files.write_back().map_err(|e| e.to_string())?;
    }

    Ok(fixed_total)
}

// #[cfg(test)]
// mod tests {
// comment out for refactoring
// }
