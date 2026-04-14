use deb822_lossless::{Deb822, Paragraph};

pub fn get_paragraph_by_package(package: &str, control: &mut Deb822) -> Option<Paragraph> {
    let package = package.to_string();
    control
        .paragraphs()
        .find(|p| !p.contains_key("Source") && p.get("Package").as_ref() == Some(&package))
}
