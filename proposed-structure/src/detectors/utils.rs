use deb822_lossless::{Deb822, Paragraph};

use crate::PackageType;

pub fn get_source_paragraph(content: &Deb822) -> Option<Paragraph> {
    content.paragraphs().find(|p| p.contains_key("Source"))
}
pub fn get_binary_paragraphs(content: &Deb822) -> Vec<Paragraph> {
    content
        .paragraphs()
        .filter(|p| !p.contains_key("Source"))
        .collect()
}

pub fn get_package_type(paragraph: &Paragraph) -> PackageType {
    if paragraph.contains_key("Source") {
        PackageType::Source
    } else {
        PackageType::Binary
    }
}
