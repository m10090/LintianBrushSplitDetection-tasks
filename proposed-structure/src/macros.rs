//! Macro to declare a detector
//!
//! This macro generates the necessary implementation and registration for a detector.

/// Macro to declare a detector
///
/// This macro generates a detector struct that implements the Detector trait
/// and registers it with the inventory for automatic discovery.
///
/// # Note
///
/// The module calling this macro MUST define a constant:
/// `const DETECTOR_NAME: &str = "...";` before invoking the macro.
///
/// # Example
/// ```ignore
/// const DETECTOR_NAME: &str = "my-detector";
///
/// declare_detector! {
///     tags: ["my-lintian-tag"],
///     detect: |files| {
///         // Detection logic here - files is &DebianFiles
///         Ok(vec![])
///     }
/// }
/// ```
#[macro_export]
macro_rules! declare_detector {
    (
        tags: [$($tag:expr),*],
        detect: $detect_fn:expr
    ) => {
        pub struct DetectorImpl;

        impl $crate::Detector for DetectorImpl {
            fn name(&self) -> &'static str {
                DETECTOR_NAME
            }

            fn tags(&self) -> &'static [&'static str] {
                &[$($tag),*]
            }

            fn detect(
                &self,
                files: &$crate::DebianFiles,
            ) -> Result<Vec<$crate::DetectedIssue>, $crate::DetectorError> {
                let detect_fn: fn(&$crate::DebianFiles) -> Result<Vec<$crate::DetectedIssue>, $crate::DetectorError> = $detect_fn;
                detect_fn(files)
            }
        }

        impl Default for DetectorImpl {
            fn default() -> Self {
                DetectorImpl
            }
        }

        inventory::submit! {
            $crate::DetectorRegistration {
                name: DETECTOR_NAME,
                lintian_tags: &[$($tag),*],
                create: || Box::new(DetectorImpl),
            }
        }
    };
}

/// Macro to declare a fixer
///
/// This macro generates a fixer struct that implements the Fixer trait
/// and registers it with inventory for automatic discovery.
///
/// # Example
/// ```ignore
/// declare_fixer! {
///     name: "my-fixer",
///     tag: "my-lintian-tag",
///     description: "Fixes my lintian tag",
///     apply: |issues, files| {
///         let _ = (issues, files);
///         Ok(0)
///     }
/// }
/// ```
#[macro_export]
macro_rules! declare_fixer {
    (
        name: $name:expr,
        tag: $tag:expr,
        description: $description:expr,
        apply: $apply_fn:expr
    ) => {
        pub struct FixerImpl;

        impl $crate::Fixer for FixerImpl {
            fn name(&self) -> &'static str {
                $name
            }

            fn tag(&self) -> &'static str {
                $tag
            }

            fn description(&self) -> &'static str {
                $description
            }

            fn apply(
                &self,
                issues: &[$crate::DetectedIssue],
                files: &mut $crate::DebianFilesMut,
            ) -> Result<usize, $crate::FixerError> {
                let apply_fn: fn(
                    &[$crate::DetectedIssue],
                    &mut $crate::DebianFilesMut,
                ) -> Result<usize, $crate::FixerError> = $apply_fn;
                apply_fn(issues, files)
            }
        }

        impl Default for FixerImpl {
            fn default() -> Self {
                FixerImpl
            }
        }

        inventory::submit! {
            $crate::FixerRegistration {
                name: $name,
                lintian_tag: $tag,
                description: $description,
                create: || Box::new(FixerImpl),
            }
        }
    };
}
