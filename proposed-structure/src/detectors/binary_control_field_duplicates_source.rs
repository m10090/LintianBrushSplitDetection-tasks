//! Detector for binary fields that duplicate source fields in debian/control
//!
//! Detects when a binary package paragraph contains a field with the same
//! name and value as the source paragraph, which is redundant.

use crate::{
    DebianFiles, DetectedIssue, DetectorError,
    detectors::utils::{get_binary_paragraphs, get_package_type, get_source_paragraph},
};
use std::collections::HashMap;
const DETECTOR_NAME: &str = "binary-control-field-duplicates-source";

fn run(
    files: &DebianFiles,
) -> Result<Vec<(DetectedIssue, Option<Box<dyn FnOnce()>>)>, DetectorError> {
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
                let paragraph = binary.clone();
                let package_type = get_package_type(&paragraph);

                issues.push(create_issue!(
                    package: package_name,
                    package_type: package_type,
                    line: Some(line_number),
                    description: format!(
                        "Field '{}' in binary package '{}' duplicates source paragraph value '{}'",
                        key,
                        package_name
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string()),
                        value.trim()
                        ),
                    field: Some(key.clone()),
                    tag: "installable-field-mirrors-source",
                    apply: move || {
                        let mut paragraph = paragraph;
                        paragraph.remove(&key);
                    }
                ));
            }
        }
    }

    Ok(issues)
}

// declare_detector! {
//     tags: ["binary-control-field-duplicates-source"],
//     detect: run
// }

// #[cfg(test)]
// mod tests {
//     // Tests are commented out for refactoring
// }
