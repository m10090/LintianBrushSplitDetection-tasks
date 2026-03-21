# hello-debian test fixture

This fixture intentionally contains control-file issues covered by current detectors/fixers:

- `debian-control-has-empty-field` (`Maintainer`, `Provides`)
- `installable-field-mirrors-source` (`Priority` duplicated from source in binary paragraph)
- `cute-field` (`HomePage` should be `Homepage`)
- `debian-control-has-unusual-field-spacing` (`Build-Depends:  ...`, `Architecture:  any`)

It is used by integration tests under `src/lib.rs`.
