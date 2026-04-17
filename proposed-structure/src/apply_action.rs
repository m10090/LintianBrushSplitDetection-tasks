use deb822_lossless::Paragraph;
use debian_analyzer::control::TemplatedControlEditor;

use crate::{Action, DebianFilesMut, DetectedIssue, PackageType};

/// Apply planned actions to the mutable Debian files.
///
/// Returns the number of applied actions.
pub fn apply_action(files: &mut DebianFilesMut, issues: &[DetectedIssue]) -> usize {
    let Some(control) = files.control.as_mut() else {
        return 0;
    };

    let editor = &mut control.editor;
    let mut applied = 0usize;

    for issue in issues.iter() {
        let Some(action) = issue.action.clone() else {
            continue;
        };

        match action {
            Action::UpdateKey {
                package,
                package_type,
                old_key,
                new_key,
            } => match package_type {
                PackageType::Source => {
                    if let Some(mut source) = editor.source() {
                        let paragraph = source.as_mut_deb822();
                        applied += paragraph.rename(old_key.as_str(), new_key.as_str()) as usize; 
                    }
                }
                PackageType::Binary => {
                    if let Some(pkg) = package
                        && let Some(mut paragraph) = find_binary(editor, pkg.as_str())
                    {
                        applied += paragraph.rename(old_key.as_str(), new_key.as_str()) as usize; 
                    }
                }
            },

            Action::DeleteKey {
                package,
                package_type,
                key,
            } => match package_type {
                PackageType::Source => {
                    if let Some(mut source) = editor.source() {
                        let paragraph = source.as_mut_deb822();
                        paragraph.remove(key.as_str()); 
                        applied += 1;
                    }
                }
                PackageType::Binary => {
                    if let Some(pkg) = package
                        && let Some(mut paragraph) = find_binary(editor, pkg.as_str())
                    {
                        paragraph.remove(key.as_str());
                        applied += 1;
                    }
                }
            },

            Action::UpdateValue {
                package,
                package_type,
                key,
                current_value,
                new_value,
            } => match package_type {
                PackageType::Source => {
                    if let Some(mut source) = editor.source()
                        && let paragraph = source.as_mut_deb822()
                        && let Some(entry) = paragraph.get_entry(key.as_str())
                        && entry.value() == current_value
                    {
                        paragraph.set(&key, &new_value);
                        applied += 1;
                    }
                }
                PackageType::Binary => {
                    if let Some(pkg) = package
                        && let Some(mut paragraph) = find_binary(editor, pkg.as_str())
                        && paragraph.get_entry(key.as_str()).is_some()
                    // check if the field is
                    // here
                    {
                        paragraph.set(&key, &new_value);
                        applied += 1;
                    }
                }
            },

            Action::NormalizeFieldSpacing {
                package,
                package_type,
                key,
            } => match package_type {
                PackageType::Source => {
                    if let Some(mut source) = editor.source()
                        && let paragraph = source.as_mut_deb822()
                        && let Some(mut entry) = paragraph.get_entry(key.as_str())
                    {
                        entry.normalize_field_spacing();
                        applied += 1;
                    }
                }
                PackageType::Binary => {
                    if let Some(pkg) = package
                        && let Some(paragraph) = find_binary(editor, pkg.as_str())
                        && let Some(mut entry) = paragraph.get_entry(key.as_str())
                    {
                        entry.normalize_field_spacing();
                        applied += 1;
                    }
                }
            },
        }
    }

    applied
}

fn find_binary(editor: &TemplatedControlEditor, package_name: &str) -> Option<Paragraph> {
    for binary in editor.binaries() {
        if binary.get("Package").as_deref() == Some(package_name) {
            return Some(binary.as_deb822().clone());
        }
    }
    None
}
