//! Fixer for unusual field spacing in debian/control

use crate::{
    DebianFilesMut, DetectedIssue, FixerError, PackageType, fixers::utils::get_paragraph_by_package,
};

fn run(issues: &[DetectedIssue], files: &mut DebianFilesMut) -> Result<usize, FixerError> {
    let Some(control) = files.control.as_mut() else {
        return Ok(0);
    };

    let editor = &mut control.editor;
    let mut fixed = 0usize;

    for DetectedIssue {
        field,
        package_type,
        package,
        ..
    } in issues
    {
        let paragraph = if package_type == &PackageType::Source {
            editor.source().map(|s| s.as_deb822().clone())
        } else if let Some(package) = package {
            get_paragraph_by_package(package.as_str(), editor.as_mut_deb822())
        } else {
            // should report this as implementation error
            continue;
        };

        let Some(paragraph) = paragraph else {
            // should report this as implementation error
            continue;
        };

        let Some(field) = field else {
            // should report this as implementation error
            continue;
        };

        let Some(mut entry) = paragraph.get_entry(field.as_str()) else {
            // should report this as implementation error
            continue;
        };
        entry.normalize_field_spacing();
        fixed += 1;
    }

    Ok(fixed)
}

declare_fixer! {
    name: "debian-control-has-unusual-field-spacing",
    tag: "debian-control-has-unusual-field-spacing",
    description: "Normalizes spacing after field separators in debian/control.",
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
    fn test_fix_unusual_spacing() {
        let content = "Source: test\nBuild-Depends:  debhelper\n";
        let temp_dir = setup_control_file(content);

        let issues = vec![DetectedIssue {
            tag: "debian-control-has-unusual-field-spacing".to_string(),
            description: "".to_string(),
            package: None,
            package_type: PackageType::Source,
            line: Some(2),
            field: Some("Build-Depends".to_string()),
        }];

        let mut files = load_debian_files_mut(temp_dir.path()).unwrap();
        let fixer = FixerImpl;
        let fixed = fixer.apply(&issues, &mut files).unwrap();
        files.write_back().unwrap();

        assert_eq!(fixed, 1);
        let updated = fs::read_to_string(temp_dir.path().join("debian/control")).unwrap();
        assert!(updated.contains("Build-Depends: debhelper"));
        assert!(!updated.contains("Build-Depends:  debhelper"));
    }
}
