# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.2] - 2026-10-21

### Added
- **Native Windows File & Folder Dialogs**: Integrated `rfd` for instant, non-blocking native Explorer dialogs in Add Startup Item, Install Tracker, and Portable Apps scanner.
- **Automated GitHub Release CI/CD**: Workflow `.github/workflows/release.yml` with automated builds and SHA-256 checksum generation on release tags.
- **Process Auto-Termination for Updates**: Winget updater automatically detects and gracefully stops active application processes before running in-place upgrades.

### Changed
- **Modular Frontend Architecture**: Refactored monolithic codebase into clean domain modules (`modals/`, `tabs/`, `cleaner.rs`, `types.rs`, `app.rs`).
- **Asynchronous Winget Manager**: Opening Winget Manager is now non-blocking with background thread scanning and UI spinner.
- **PATH Sanitizer UI Layout**: Expanded modal width to 780px and separated selection buttons from item counters to eliminate text overlaps.

### Fixed
- **Dead Package Removal (`0x800401f5 : Application not found`)**: Added cascading registry cleanup with UAC elevation support (`ShellExecuteExW` `runas`) when an uninstaller executable is missing from disk.
- **Winget Upgrade Error (Exit Code 1)**: Fixed Inno Setup silent update failures caused by active tray/background processes (e.g., ShareX).
- **Console Encoding**: Added `CP_OEMCP` (CP866) console decoding to eliminate garbled error messages from Winget/CLI tools on localized Windows installations.

---

## [1.0.1] - 2026-10-20

### Fixed
- Settings buttons alignment and layout responsiveness in windowed mode.
- System clean and dev cache deep scanning optimizations.

---

## [1.0.0] - 2026-10-20

### Added
- Initial public release of PurgeCraft uninstaller and Windows optimizer.
- Win32 and UWP app scanning, leftover deep scanner, services and zombie process manager.
