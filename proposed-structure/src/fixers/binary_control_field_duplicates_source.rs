//! Fixer for duplicated source fields in binary paragraphs

use crate::{DebianFilesMut, DetectedIssue, FixerError, fixers::utils::get_pargraph_by_package};

fn run(issues: &[DetectedIssue], files: &mut DebianFilesMut) -> Result<usize, FixerError> {
    let Some(control) = files.control.as_mut() else {
        return Ok(0);
    };

    let editor = &mut control.editor;
    let control = editor.as_mut_deb822();
    let mut fixed = 0;
    // get the fileds needed
    for DetectedIssue { package, field, .. } in issues {
        let Some(package) = package else {
            // should report this as implementation error
            continue;
        };
        let Some(mut paragraph) = get_pargraph_by_package(package, control) else {
            continue;
        };
        let Some(field) = field else {
            // should report this as implementation error
            continue;
        };
        paragraph.remove(field);
        fixed += 1;
    }

    Ok(fixed)
}

declare_fixer! {
    name: "binary-control-field-duplicates-source",
    tag: "installable-field-mirrors-source",
    description: "Removes binary fields that duplicate values from source paragraph.",
    apply: run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fixer, PackageType, load_debian_files_mut};
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
    fn test_fix_duplicate_field() {
        let content = "Source: test\nPriority: optional\n\nPackage: test\nPriority: optional\nDescription: d\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "installable-field-mirrors-source".to_string(),
            description: "".to_string(),
            package: Some("test".to_string()),
            package_type: PackageType::Binary,
            line: Some(5),
            field: Some("Priority".to_string()),
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(!updated.contains("Package: test\nPriority: optional"));
    }
}
