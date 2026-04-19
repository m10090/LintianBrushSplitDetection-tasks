use debian_analyzer::control::TemplatedControlEditor;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_delete_and_edit_adjacent_entries() {
    let temp_dir = TempDir::new().unwrap();
    let control_path = temp_dir.path().join("control");
    fs::write(
        &control_path,
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    )
    .unwrap();

    let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();

    let mut p1 = editor.as_deb822().paragraphs().next().unwrap();
    let mut p1_stale = p1.clone();

    p1.remove("Section");
    p1_stale.insert("Priority", "required");

    editor.commit().unwrap();

    let content = fs::read_to_string(&control_path).unwrap();
    // Prove stale edit duplicate field
    assert_eq!(
        content,
        "Source: foo\nPriority: optional\nPriority: required\n\nPackage: foo-bin\nArchitecture: any\n"
    );
}

#[test]
fn test_delete_wins_over_edit_on_same_entry() {
    let temp_dir = TempDir::new().unwrap();
    let control_path = temp_dir.path().join("control");
    fs::write(
        &control_path,
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    )
    .unwrap();

    let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();

    let mut p1 = editor.as_deb822().paragraphs().next().unwrap();
    let mut p1_stale = p1.clone();

    p1.remove("Section");
    p1_stale.insert("Section", "web");

    editor.commit().unwrap();

    let content = fs::read_to_string(&control_path).unwrap();
    // Prove delete NOT win. Stale node resurrects field.
    assert_eq!(
        content,
        "Source: foo\nPriority: optional\nSection: web\n\nPackage: foo-bin\nArchitecture: any\n"
    );
}

#[test]
fn test_edit_paragraph_stale_reference() {
    let temp_dir = TempDir::new().unwrap();
    let control_path = temp_dir.path().join("control");
    fs::write(
        &control_path,
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    )
    .unwrap();

    let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();

    let mut p1 = editor.as_deb822().paragraphs().nth(0).unwrap();
    let mut p2 = editor.as_deb822().paragraphs().nth(1).unwrap();

    p1.insert("Maintainer", "me");
    p2.insert("Depends", "libc6");

    editor.commit().unwrap();

    let content = fs::read_to_string(&control_path).unwrap();
    // Both edits apply, but p2 was technically stale relative to entire tree root.
    // This happens to work cleanly because paragraphs are separated by blank lines,
    // but still relies on rowan tracking offset shifts correctly.
    assert_eq!(
        content,
        "Source: foo\nSection: utils\nPriority: optional\nMaintainer: me\n\nPackage: foo-bin\nArchitecture: any\nDepends: libc6\n"
    );
}

#[test]
fn test_edit_stale_entry_value() {
    let temp_dir = TempDir::new().unwrap();
    let control_path = temp_dir.path().join("control");
    fs::write(
        &control_path,
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    )
    .unwrap();

    let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();

    let mut p1 = editor.as_deb822().paragraphs().next().unwrap();
    let mut p1_stale = p1.clone();

    p1.insert("Section", "web");
    p1_stale.insert("Section", "base");

    editor.commit().unwrap();

    let content = fs::read_to_string(&control_path).unwrap();
    // Prove stale edit makes duplicate field instead of overwrite
    assert_eq!(
        content,
        "Source: foo\nSection: utils\nPriority: optional\nSection: web\nSection: base\n\nPackage: foo-bin\nArchitecture: any\n"
    );
}

#[test]
fn test_delete_paragraph_edit_stale_entry() {
    let temp_dir = TempDir::new().unwrap();
    let control_path = temp_dir.path().join("control");
    fs::write(
        &control_path,
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    )
    .unwrap();

    let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();

    let mut p1_stale = editor.as_deb822().paragraphs().next().unwrap();
    let mut p1_mut = editor.as_deb822().paragraphs().next().unwrap();

    p1_mut.remove("Source");
    p1_mut.remove("Section");
    p1_mut.remove("Priority");

    p1_stale.insert("Source", "bar");

    editor.commit().unwrap();

    let content = fs::read_to_string(&control_path).unwrap();
    // Prove stale edit on cleared paragraph adds new text
    assert_eq!(
        content,
        "Source: bar\n\nPackage: foo-bin\nArchitecture: any\n"
    );
}

#[test]
fn test_try_to_delete_paragraph_and_edit_it() {
    let temp_dir = TempDir::new().unwrap();
    let control_path = temp_dir.path().join("control");
    fs::write(
        &control_path,
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    )
    .unwrap();

    let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();

    let mut p2 = editor.as_deb822().paragraphs().nth(1).unwrap();
    let mut p2_stale = p2.clone();

    let keys: Vec<String> = p2.keys().map(|k| k.to_string()).collect();
    for k in keys {
        p2.remove(&k);
    }

    p2_stale.insert("Architecture", "all");
    p2_stale.insert("Depends", "libc6");

    editor.commit().unwrap();

    let content = fs::read_to_string(&control_path).unwrap();
    // Prove stale edit resurrects fields in ghost paragraph
    assert_eq!(
        content,
        "Source: foo\nSection: utils\nPriority: optional\n\nArchitecture: all\nDepends: libc6\n"
    );
}
#[test]
fn test_try_to_delete_paragraph_and_edit_it_while_tree_is_droped() {
    let temp_dir = TempDir::new().unwrap();
    let control_path = temp_dir.path().join("control");
    fs::write(
        &control_path,
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    )
    .unwrap();

    let mut p2;
    let mut p2_stale;
    {
        let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();

        p2 = editor.as_deb822().paragraphs().nth(1).unwrap();
        p2_stale = p2.clone();
    }

    let keys: Vec<String> = p2.keys().map(|k| k.to_string()).collect();
    for k in keys {
        p2.remove(&k);
    }

    p2_stale.insert("Architecture", "all");
    p2_stale.insert("Depends", "libc6");

    let editor = TemplatedControlEditor::new(control_path.clone(), false).unwrap();
    editor.commit().unwrap();
    
    assert_eq!(
        fs::read_to_string(&control_path).unwrap(),
        "Source: foo\nSection: utils\nPriority: optional\n\nPackage: foo-bin\nArchitecture: any\n",
    );

}
