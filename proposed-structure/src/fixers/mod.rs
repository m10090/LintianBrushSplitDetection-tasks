//! Fixers module - contains all fixer implementations

pub mod binary_control_field_duplicates_source;
pub mod debian_control_has_empty_field;
pub mod debian_control_has_unusual_field_spacing;
pub mod field_name_typo_in_control;

pub use binary_control_field_duplicates_source::FixerImpl as BinaryControlFieldDuplicatesSourceFixer;
pub use debian_control_has_empty_field::FixerImpl as DebianControlHasEmptyFieldFixer;
pub use debian_control_has_unusual_field_spacing::FixerImpl as DebianControlHasUnusualFieldSpacingFixer;
pub use field_name_typo_in_control::FixerImpl as FieldNameTypoInControlFixer;
