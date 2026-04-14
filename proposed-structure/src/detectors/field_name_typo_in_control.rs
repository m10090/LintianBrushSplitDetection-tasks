//! Detector for field name typos (case mismatches) in debian/control
//!
//! Detects fields where the casing doesn't match the canonical Debian field names,
//! e.g., "HomePage" instead of "Homepage".

use crate::{
    create_issue, DebianFiles, DetectedIssue, DetectorError, detectors::utils::get_package_type,
};
use std::collections::HashSet;

const DETECTOR_NAME: &str = "field-name-typo-in-control";
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

fn run(
    files: &DebianFiles,
) -> Result<Vec<(DetectedIssue, Option<Box<dyn FnOnce() -> ()>>)>, DetectorError> {
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

                let mut paragraph_clone = paragraph.clone();
                let key_clone = key.to_string();
                let correct_field_clone = correct_field.to_string();

                let issue = create_issue!(
                    package: package_name.clone(),
                    package_type: package_type.clone(),
                    line: Some(line_number),
                    description: description,
                    field: Some(key.to_string()),
                    tag: "cute-field",
                    apply: move || {
                        paragraph_clone.rename(&key_clone, &correct_field_clone);
                    }
                );
                issues.push(issue);
            }
        }
    }

    Ok(issues)
}

// declare_detector! {
//     tags: ["cute-field"],
//     detect: run
// }

// #[cfg(test)]
// mod tests {
//     // Tests are commented out for refactoring
// }
