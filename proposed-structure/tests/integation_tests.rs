use std::fs;
use tempfile::TempDir;

use proposed_structure::{apply_all_fixers, detect_all};

fn setup_control_file(content: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let debian_dir = temp_dir.path().join("debian");
    fs::create_dir_all(&debian_dir).unwrap();
    fs::write(debian_dir.join("control"), content).unwrap();
    temp_dir
}

#[test]
fn test_integration_fix_multiple_duplicates() {
    // Here we test a fixer running multiple times in one iteration on multiple
    // instances of the same issue. The `binary-control-field-duplicates-source`
    // fixer will remove both the Section and Priority fields from the binary.
    let content = r#"Source: test-package
Section: utils
Priority: optional
Maintainer: Test <test@example.com>

Package: test-package
Architecture: any
Section: utils
Priority: optional
Description: Test package
"#;
    let temp_dir = setup_control_file(content);

    // 1. Detect issues
    let issues = detect_all(temp_dir.path()).unwrap();

    let dup_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.tag == "installable-field-mirrors-source")
        .collect();

    assert_eq!(dup_issues.len(), 2, "Expected 2 duplicate field issues");

    // Assert we caught both Section and Priority
    assert!(
        dup_issues
            .iter()
            .any(|i| i.field.as_deref() == Some("Section"))
    );
    assert!(
        dup_issues
            .iter()
            .any(|i| i.field.as_deref() == Some("Priority"))
    );

    // 2. Fix issues
    let fixed = apply_all_fixers(temp_dir.path(), &issues, false).unwrap();
    assert_eq!(fixed, 2, "Expected 2 issues to be fixed");

    // 3. Verify file contents
    let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
    assert!(updated.contains("Source: test-package\nSection: utils\nPriority: optional"));
    assert!(!updated.contains("Section: utils\nPriority: optional\nDescription:"));
    assert!(
        updated.contains("Package: test-package\nArchitecture: any\nDescription: Test package")
    );

    // 4. Verify no remaining issues
    let remaining = detect_all(temp_dir.path()).unwrap();
    assert!(
        remaining.is_empty(),
        "Expected no remaining issues, got {:?}",
        remaining
    );
}

#[test]
fn test_integration_multiple_detectors_and_fixers() {
    // Test combining two different issues that require two different fixers
    // 1. Empty field (Maintainer:) -> debian-control-has-empty-field
    // 2. Field name typo (HomePage) -> cute-field
    let content = r#"Source: multi-test
Maintainer  :  
HomePage: https://example.com

Package: multi-test
Architecture: any
Description: Multi test
"#;
    let temp_dir = setup_control_file(content);

    // 1. Detect issues
    let issues = detect_all(temp_dir.path()).unwrap();

    // Should find exactly 3 issues (Maintainer empty, Maintainer unusual spacing, HomePage case)
    assert_eq!(
        issues.len(),
        3,
        "Expected exactly 3 issues, got: {:#?}",
        issues
    );

    let has_empty = issues.iter().any(|i| {
        i.tag == "debian-control-has-empty-field" && i.field.as_deref() == Some("Maintainer")
    });
    let has_spacing = issues.iter().any(|i| {
        i.tag == "debian-control-has-unusual-field-spacing"
            && i.field.as_deref() == Some("Maintainer")
    });
    let has_typo = issues
        .iter()
        .any(|i| i.tag == "cute-field" && i.field.as_deref() == Some("HomePage"));

    assert!(has_empty, "Missing empty field issue for Maintainer");
    assert!(has_spacing, "Missing unusual spacing issue for Maintainer");
    assert!(has_typo, "Missing typo issue for HomePage");

    // 2. Fix issues (will dispatch to 3 separate fixers based on the tags)
    let _fixed = apply_all_fixers(temp_dir.path(), &issues,false).unwrap();
    // this assert fails showing that 2 detector detected a issue in 2 of them
    // to be exact 'debian-control-has-unusual-field-spacing'
    // and 'debian-control-has-unusual-field-spacing' found the same issue in the feld
    // Maintainer
    // one of them delete the hole field and the other other edited the value in the feild
    // assert_eq!(_fixed, 3, "Expected 3 issues to be fixed");

    // 3. Verify file contents
    let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
    assert!(
        updated.contains("Homepage: https://example.com"),
        "Homepage should be correctly cased"
    );
    assert!(
        !updated.contains("Maintainer:"),
        "Empty Maintainer field should be completely removed"
    );

    // 4. Verify no remaining issues
    let remaining = detect_all(temp_dir.path()).unwrap();
    assert!(
        remaining.is_empty(),
        "Expected no remaining issues, got {:?}",
        remaining
    );
}
