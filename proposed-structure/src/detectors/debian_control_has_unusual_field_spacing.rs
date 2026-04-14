//! Detector for unusual field spacing in debian/control
//!
//! Checks for things like double spaces after a colon or tabs where
//! spaces are expected, before the field value starts.
//! Uses the deb822-lossless `normalize_field_spacing()` method to detect
//! fields that would need spacing normalization.

use crate::{DebianFiles, DetectedIssue, DetectorError, detectors::utils::get_package_type, PackageType};

const DETECTOR_NAME: &str = "debian-control-has-unusual-field-spacing";

fn run(
    files: &DebianFiles,
) -> Result<Vec<(DetectedIssue, Option<Box<dyn FnOnce() -> ()>>)>, DetectorError> {
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
            let original_text = entry.to_string();

            // We must temporarily modify the entry to see if normalization changes it
            // (since deb822-lossless doesn't have a check-only method)
            if entry.normalize_field_spacing() {
                let new_text = entry.to_string();
                if original_text != new_text {
                    let line_number = entry.line() + 1;
                    if let Some(key) = entry.key() {
                        let key_str = key.to_string();
                        let mut entry_clone = entry.clone();

                        issues.push(create_issue!(
                            package: package_name.clone(),
                            package_type: package_type.clone(),
                            line: Some(line_number),
                            description: format!(
                                "Field '{}' has unusual spacing [{}:{}]",
                                key,
                                control.path.display(),
                                line_number
                            ),
                            field: Some(key_str),
                            tag: "debian-control-has-unusual-field-spacing",
                            apply: move || {
                                entry_clone.normalize_field_spacing();
                            }
                        ));
                    }
                }
            }
        }
    }

    Ok(issues)
}

// declare_detector! {
//     tags: ["debian-control-has-unusual-field-spacing"],
//     detect: run
// }

// #[cfg(test)]
// mod tests {
//     // Tests commented out
// }
