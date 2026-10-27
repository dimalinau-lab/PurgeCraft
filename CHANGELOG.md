# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.4] - 2026-10-27

### Fixed
- **Chromium Keyboard Shortcuts Conflict**: Disabled `accesskit` in `eframe` to prevent unwanted Windows UI Automation (UIA) registration. Resolves an issue where running PurgeCraft in the background caused Chromium-based browsers (Chrome, Edge, Brave, Yandex Browser) to swallow/block all keyboard shortcuts (`Ctrl+T`, `Ctrl+W`, `Ctrl+C`, `Ctrl+V`, `Ctrl+1..9`, etc.).

### Changed
- **Binary Size & Performance Optimization**: Stripped over 1,000 lines of unused accessibility and UIA boilerplate dependencies, reducing the release binary size by over 400 KB and speeding up application cold start.

---

## [1.0.3] - 2026-10-21

### Added
- **Comprehensive RU/EN Localization**: Full bilingual support across the entire interface (Settings, Sidebar, Tools, System Cleaners, Developer Caches, Services Manager, Installation Tracker, Uninstallation Wizard, Dialogs, and Updater).
- **Live Reactive Language Preview**: Switching language in Settings immediately updates all labels, tabs, descriptions, and buttons without requiring an application restart.
- **Global Keyboard Shortcuts**: Standard desktop navigation shortcuts: F5 (refresh), Del (uninstall selected), Ctrl+F (focus search), Ctrl+1/2/3/4 (switch tabs), Ctrl+, (open settings), and Esc (clear search / close dialogs).
- **Localized Size Units & Metrics**: File sizes, item counts, and summaries dynamically format using language-specific units (B, KB, MB, GB vs B, KB, MB, GB).

### Changed
- **CI/CD Release Automation**: Updated GitHub Actions release workflow to use clean tag-based release titles.

### Fixed
- **Untranslated Utility Modals**: Resolved remaining hardcoded strings in GPU Shader cleaner, PATH sanitizer, Portable apps scanner, VC++/Java runtime duplicate detector, Winget manager, and reboot cleanup dialogs.

---

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
