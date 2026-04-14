//! Fixer for empty fields in debian/control

use crate::{
    DebianFilesMut, DetectedIssue, FixerError, PackageType, fixers::utils::get_pargraph_by_package,
};


fn run(issues: &[DetectedIssue], files: &mut DebianFilesMut) -> Result<usize, FixerError> {
    let Some(control) = files.control.as_mut() else {
        return Ok(0);
    };

    let editor = &mut control.editor;
    let mut fixed = 0usize;

    for DetectedIssue {
        package_type,
        field,
        package,
        ..
    } in issues
    {
        let Some(field) = field else {
            // should report this as implementation error
            continue;
        };

        if package_type == &PackageType::Source {
            let Some(mut source) = editor.source() else {
                // should report this as implementation error
                continue;
            };
            let paragraph = source.as_mut_deb822();

            paragraph.remove(field.as_str());
            fixed += 1;
            continue;
        }

        let Some(package) = package else {
            // should report this as implementation error
            continue;
        };
        let Some(mut paragraph) = get_pargraph_by_package(package, editor.as_mut_deb822()) else {
            continue;
        };
        paragraph.remove(field);
        fixed += 1;
    }

    Ok(fixed)
}

declare_fixer! {
    name: "debian-control-has-empty-field",
    tag: "debian-control-has-empty-field",
    description: "Removes empty fields from debian/control paragraphs.",
    apply: run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fixer, load_debian_files_mut};
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
    fn test_fix_empty_source_field() {
        let content = "Source: test\nMaintainer:\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "debian-control-has-empty-field".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(2),
            field: Some("Maintainer".to_string()),
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(!updated.contains("Maintainer:"));
    }
}
