//! Detector for dh-systemd in Build-Depends* fields
//!
//! Emits UpdateValue actions for existing Build-Depends* fields that contain
//! dh-systemd. The action's new_value will have dh-systemd removed and the
//! Build-Depends updated to ensure a minimum debhelper version when applicable.

use crate::{DebianFiles, DetectedIssue, DetectorError, PackageType};
use debian_control::lossless::relations::Relations;
use debversion::Version;

use super::utils::get_source_paragraph;

const MINIMUM_DEBHELPER_VERSION: &str = "9.20160709";

fn run(files: &DebianFiles) -> Result<Vec<DetectedIssue>, DetectorError> {
    let Some(control) = &files.control else {
        return Ok(vec![]);
    };

    let source = match get_source_paragraph(control.content) {
        None => return Ok(vec![]),
        Some(p) => p,
    };

    let mut issues = Vec::new();

    // First pass: detect dh-systemd in any Build-Depends* field and prepare
    // per-field new_value (we only emit UpdateValue when the field exists).
    let mut will_change = false;
    let mut per_field_new: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for field_name in ["Build-Depends", "Build-Depends-Indep", "Build-Depends-Arch"] {
        if let Some(value) = source.get(field_name) {
            if value.trim().is_empty() {
                continue;
            }

            let (mut relations, _errors) = Relations::parse_relaxed(&value, true);

            let has_dh_systemd = relations.entries().any(|e| {
                e.relations()
                    .any(|r| r.try_name().as_deref() == Some("dh-systemd"))
            });

            if has_dh_systemd {
                // Drop dh-systemd from this relations
                relations.drop_dependency("dh-systemd");
                per_field_new.insert(field_name.to_string(), relations.to_string());
                will_change = true;
            }
        }
    }

    if !will_change {
        return Ok(vec![]);
    }

    // Ensure minimum debhelper version in Build-Depends
    let build_dep_current = source.get("Build-Depends").unwrap_or_default();
    let (mut build_rel, _errors) = Relations::parse_relaxed(&build_dep_current, true);
    // Remove dh-systemd from Build-Depends as well before ensuring minimum debhelper.
    build_rel.drop_dependency("dh-systemd");
    let minimum_version: Version = MINIMUM_DEBHELPER_VERSION.parse().unwrap();
    build_rel.ensure_minimum_version("debhelper", &minimum_version);

    // Create DetectedIssue entries. For Build-Depends, prefer final_build_dep;
    // for other fields use the per_field_new value.
    for entry in source.entries() {
        if let Some(key) = entry.key() {
            if key == "Build-Depends" {
                // If Build-Depends entry exists, always emit an UpdateValue to
                // ensure minimum debhelper and remove dh-systemd where present.
                issues.push(DetectedIssue {
                    tag: "build-depends-on-obsolete-package".to_string(),
                    description: format!(
                        "Remove dh-systemd and ensure debhelper minimum in '{}'",
                        key
                    ),
                    package: None,
                    package_type: PackageType::Source,
                    line: Some(entry.line() + 1),
                    field: Some(key.to_string()),
                    action: Some(crate::Action::UpdateRelation {
                        package: None,
                        package_type: PackageType::Source,
                        key: key.to_string(),
                        remove: vec!["dh-systemd".parse().unwrap()],
                        add: vec![format!("debhelper (>= {})", MINIMUM_DEBHELPER_VERSION)
                            .parse()
                            .unwrap()],
                    }),
                });
            } else if per_field_new.contains_key(&key) {
                // Non Build-Depends fields where dh-systemd was present
                issues.push(DetectedIssue {
                    tag: "build-depends-on-obsolete-package".to_string(),
                    description: format!("Remove dh-systemd from '{}'", key),
                    package: None,
                    package_type: PackageType::Source,
                    line: Some(source.get_entry(&key).map(|e| e.line() + 1).unwrap_or(0)),
                    field: Some(key.to_string()),
                    action: Some(crate::Action::UpdateRelation {
                        package: None,
                        package_type: PackageType::Source,
                        key: key.to_string(),
                        remove: vec!["dh-systemd".parse().unwrap()],
                        add: vec![],
                    }),
                });
            }
        }
    }

    Ok(issues)
}

declare_detector! {
    name: "build-depends-on-obsolete-package",
    tags: ["build-depends-on-obsolete-package"],
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
    fn test_detect_and_prepare_update() {
        let content = r#"Source: mypackage
Build-Depends: debhelper (>= 9), dh-systemd

Package: mypackage
Architecture: any
"#;

        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();
        assert!(!issues.is_empty());

        // Find the Build-Depends issue
        let bd_issue = issues
            .iter()
            .find(|i| i.field.as_deref() == Some("Build-Depends"))
            .unwrap();
        assert_eq!(bd_issue.tag, "build-depends-on-obsolete-package");
        assert!(bd_issue.action.is_some());

        if let Some(crate::Action::UpdateRelation { remove, add, .. }) = &bd_issue.action {
            // ensure remove contains dh-systemd
            assert!(remove.iter().any(|r| r.name == "dh-systemd"), "expected remove to include dh-systemd");

            // ensure add contains debhelper minimum
            assert!(add.iter().any(|r| r.name == "debhelper"), "expected add to contain debhelper");
        } else {
            panic!("Expected UpdateRelation action");
        }
    }

    #[test]
    fn test_no_dh_systemd() {
        let content = r#"Source: mypackage
Build-Depends: debhelper (>= 9)

Package: mypackage
Architecture: any
"#;

        let temp_dir = setup_control_file(content);
        let loaded = load_debian_files(temp_dir.path()).unwrap();
        let files = loaded.as_ref();
        let detector = DetectorImpl;

        let issues = detector.detect(&files).unwrap();
        assert!(issues.is_empty());
    }
}
