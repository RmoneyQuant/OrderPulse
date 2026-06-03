# Changelog

All notable changes to this project should be documented in this file.

The format is based on Keep a Changelog and the project follows Semantic Versioning.

## [0.2.47] - 2026-05-29

### Added
- Added `CONTRIBUTING.md` with setup, test, and PR guidance.
- Added repository `.gitignore` for Rust/Python build and cache artifacts.

### Changed
- Renamed Cargo package metadata from `OrderPluse` to `orderpulse` for consistency.
- Declared Cargo feature `python` to match existing `#[cfg(feature = "python")]` gates.
- Simplified `python/fastreader/__init__.py` to a clean runtime re-export surface.
- Replaced `README.md` with structured install, API, testing, troubleshooting, and maintenance docs.

### Fixed
- Replaced invalid `test_streaming.py` content with executable Python API tests.
