//! Detector for empty fields in debian/control
//!
//! Detects fields that have empty or whitespace-only values in both
//! source and binary package paragraphs.

use super::utils::get_package_type;
use crate::{DebianFiles, DetectedIssue, DetectorError, PackageType};

const DETECTOR_NAME: &str = "debian-control-has-empty-field";

fn run(
    files: &DebianFiles,
) -> Result<Vec<(DetectedIssue, Option<Box<dyn FnOnce() -> ()>>)>, DetectorError> {
    let Some(control) = &files.control else {
        return Ok(vec![]);
    };

    let mut issues = Vec::new();

    for paragraph in control.as_deb822().paragraphs() {
        let package_name = paragraph.get("Package");
        let package_type = get_package_type(&paragraph);

        for entry in paragraph.entries() {
            if let Some(key) = entry.key()
                && let value = entry.value()
                && value.trim().is_empty()
            {
                let line_number = entry.line() + 1;
                let key_str = key.to_string();
                let paragraph_clone = paragraph.clone();

                issues.push(create_issue!(
                    package: package_name.clone(),
                    package_type: package_type.clone(),
                    line: Some(line_number),
                    description: format!(
                        "Empty field '{}' in {} package '{}' [debian/control:{}]",
                        key,
                        if package_type == PackageType::Source {
                            "source"
                        } else {
                            "binary"
                        },
                        package_name.as_deref().unwrap_or("unknown"),
                        line_number
                    ),
                    field: Some(key_str.clone()),
                    tag: "debian-control-has-empty-field",
                    apply: move || {
                        let mut paragraph = paragraph_clone;
                        paragraph.remove(&key_str);
                    }
                ));
            }
        }
    }

    Ok(issues)
}

declare_detector! {
    tags: ["debian-control-has-empty-field"],
    detect: run
}

// #[cfg(test)]
// mod tests {
//     // Tests are commented out for refactoring
// }
