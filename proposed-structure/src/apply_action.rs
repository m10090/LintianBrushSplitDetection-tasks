use deb822_lossless::Paragraph;
use debian_analyzer::control::TemplatedControlEditor;

use crate::{Action, DebianFilesMut, DetectedIssue, PackageType};
use debian_control::lossless::relations::Relations;
use debversion::Version;
use debian_control::lossy::Relation;

// Return the package name portion of a relation display string.
fn relation_name_from_str(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    for (i, ch) in s.char_indices() {
        if ch == ' ' || ch == '(' || ch == ',' {
            return s[..i].to_string();
        }
    }
    s.to_string()
}

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

            Action::UpdateRelation { package, package_type, key, remove, add } => {
                match package_type {
                    PackageType::Source => {
                        if let Some(mut source) = editor.source() {
                            let paragraph = source.as_mut_deb822();
                            if let Some(entry) = paragraph.get_entry(&key) {
                                let existing = entry.value();
                                // Try to parse relations; if parsing fails, skip.
                                let (mut rel, _errors) = Relations::parse_relaxed(&existing, true);

                                // Drop requested dependencies by name (remove is Vec<Relation>)
                                for r in remove.iter() {
                                    let name = relation_name_from_str(&r.to_string());
                                    if !name.is_empty() {
                                        rel.drop_dependency(&name);
                                    }
                                }

                                // Merge add relations by concatenating serializations and reparsing
                                let add_str = add.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ");
                                if !add_str.trim().is_empty() {
                                    let base = rel.to_string();
                                    let merged = if base.trim().is_empty() {
                                        add_str.clone()
                                    } else {
                                        format!("{}, {}", base, add_str)
                                    };
                                    rel = Relations::parse_relaxed(&merged, true).0;
                                }

                                paragraph.set(&key, &rel.to_string());
                                applied += 1;
                            }
                        }
                    }
                    PackageType::Binary => {
                        if let Some(pkg) = package
                            && let Some(mut paragraph) = find_binary(editor, pkg.as_str())
                            && let Some(entry) = paragraph.get_entry(&key)
                        {
                            let existing = entry.value();
                            let (mut rel, _errors) = Relations::parse_relaxed(&existing, true);

                            for r in remove.iter() {
                                let name = relation_name_from_str(&r.to_string());
                                if !name.is_empty() {
                                    rel.drop_dependency(&name);
                                }
                            }

                            let add_str = add.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ");
                            if !add_str.trim().is_empty() {
                                let base = rel.to_string();
                                let merged = if base.trim().is_empty() {
                                    add_str.clone()
                                } else {
                                    format!("{}, {}", base, add_str)
                                };
                                rel = Relations::parse_relaxed(&merged, true).0;
                            }

                            paragraph.set(&key, &rel.to_string());
                            applied += 1;
                        }
                    }
                }
            }

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
