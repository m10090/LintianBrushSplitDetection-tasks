//! Detectors module - contains all detector implementations

pub mod binary_control_field_duplicates_source;
pub mod debian_control_has_empty_field;
pub mod debian_control_has_unusual_field_spacing;
pub mod field_name_typo_in_control;
pub mod ancient_python_version_field;
pub mod build_depends_on_obsolete_package;
    mod utils;

pub use binary_control_field_duplicates_source::DetectorImpl as BinaryControlFieldDuplicatesSourceDetector;
pub use debian_control_has_empty_field::DetectorImpl as DebianControlHasEmptyFieldDetector;
pub use debian_control_has_unusual_field_spacing::DetectorImpl as DebianControlHasUnusualFieldSpacingDetector;
pub use field_name_typo_in_control::DetectorImpl as FieldNameTypoInControlDetector;
pub use ancient_python_version_field::DetectorImpl as AncientPythonVersionFieldDetector;
pub use build_depends_on_obsolete_package::DetectorImpl as BuildDependsOnObsoletePackageDetector;
