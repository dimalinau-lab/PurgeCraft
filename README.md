<div align="center">

<img src="desktop_rust/icon.png" alt="PurgeCraft Logo" width="96" height="96" />

# PurgeCraft

**Clean-Room Native Windows Uninstaller & System Maintenance Suite in Pure Rust**  
*Переосмысление культового деинсталлятора программ в нативном приложении на чистом Rust*

[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20x64-blue.svg?style=flat-square&logo=windows)](https://microsoft.com/windows)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)](LICENSE)
[![GUI](https://img.shields.io/badge/GUI-egui%200.30-purple.svg?style=flat-square)](https://github.com/emilk/egui)

[English Documentation](#english) • [Документация на русском](#русский)

</div>

---

<a name="english"></a>
## English

A ground-up, clean-room, open-source reimagining of legendary tools like CrystalIdea Uninstall Tool, written entirely in native Rust.

4-step residual trace wizard, batch and forced uninstallation, real-time install tracker, deep multi-library Steam and Epic Games discovery, GPU shader cleaner, and PATH sanitizer — all packed into a blazing-fast, offline-first desktop application with zero Electron bloat, zero telemetry, and sub-millisecond registry access.

### Why PurgeCraft?

Most modern system utilities are either bloated web wrappers (Electron) that consume gigabytes of RAM or closed-source shareware loaded with background telemetry, subscriptions, and trial limits.

PurgeCraft is engineered in pure Rust directly against Windows Win32 APIs, offering instant startup, microsecond system response, and complete transparency.

### Core Features

- **Lightning-Fast Registry Cleaner:** Powered by native Win32 `RegDeleteTreeW` to recursively delete deep subkeys and value trees in `< 1ms` with zero shell freezing.
- **Deep Gaming Library Discovery:** Native parser for multi-drive Steam libraries (`libraryfolders.vdf`, `appmanifest_<id>.acf`) and Epic Games manifests. Correctly identifies actual game common directories and true `SizeOnDisk` rather than desktop icon cache folders.
- **Universal Software Management:** Unified catalog covering 64-bit and 32-bit Win32 desktop applications, Windows Store (UWP / AppX) packages, and standalone Portable executables.
- **4-Step Residual Leftovers Wizard:** Exhaustive residual file and registry audit across Program Files, AppData, Local, Roaming, ProgramData, and registry hives. Features parent-first sorting, instant child pruning, and automatic delayed removal on reboot (`MoveFileExW`) for locked files.
- **GPU Shader Cache Cleaner:** Scans and purges compiled graphics shader caches across NVIDIA (`DXCache`, `GLCache`, `NV_Cache`), AMD (`DxCache`), DirectX Shader Cache, and Vulkan pipelines to reclaim disk space and eliminate game stuttering.
- **Environment PATH Sanitizer:** Inspects both System and User `PATH` environment variables, highlighting dead references and redundant duplicates with one-click healing.
- **Runtime Duplicate Analyzer:** Groups Microsoft Visual C++ Redistributable families and Java Runtimes to easily identify and eliminate outdated duplicate versions.
- **Winget Integration:** Seamlessly integrates with Windows Package Manager (Winget CLI) for quiet headless uninstalls and official software update checks.
- **Install Tracker:** Performs byte-accurate system snapshots before and after running installers to track every created file and registry key for 100% complete future uninstallation.
- **Engineered Native GUI:** Built with hardware-accelerated `egui 0.30` featuring custom DPI text scaling, true graphite dark and crisp light themes, and real executable `.ico` extraction.

### Keyboard Shortcuts

| Shortcut | Description |
|---|---|
| `F5` | Refresh installed software list |
| `Ctrl + F` | Jump focus to search bar |
| `Escape` | Clear current search query |

### Build from Source

#### Prerequisites
- Windows 10 / 11 (64-bit)
- Rust 1.80+ (`stable-x86_64-pc-windows-msvc`)

```bash
git clone https://github.com/your-username/PurgeCraft.git
cd PurgeCraft
cargo build --release --bin purgecraft --manifest-path desktop_rust/Cargo.toml
```

The compiled standalone executable will be located at:  
`desktop_rust/target/release/purgecraft.exe`

---

<a name="русский"></a>
## Русский

Полностью переработанная и созданная с нуля в «чистой комнате» версия легендарного Uninstall Tool с открытым исходным кодом, написанная целиком на нативном Rust.

4-этапный мастер поиска остатков, пакетная и принудительная деинсталляция, монитор установки (Install Tracker), сканирование игр Steam на любых дисках, очистка шейдеров GPU и санитайзер системного PATH — в легковесном, полностью автономном приложении без веб-оболочек, без телеметрии и без скрытых подписок.

### Почему PurgeCraft?

Большинство современных утилит для очистки системы превратились либо в неповоротливые веб-приложения на Electron, пожирающие гигабайты оперативной памяти, либо в проприетарные платные комбайны со встроенной рекламой и навязчивыми фоновыми службами.

PurgeCraft возвращает классический подход: 100% нативный код на Rust, прямое обращение к Win32 API и реестру Windows, мгновенный запуск за доли секунды и прозрачность открытого исходного кода.

### Основные возможности

- **Молниеносная зачистка реестра:** Нативный Win32 API вызов `RegDeleteTreeW` рекурсивно удаляет любые разветвленные ветки и ключи менее чем за `1 мс` без подвисаний и всплывающих окон командной строки.
- **Глубокий парсер библиотек Steam и Epic:** Сканирование библиотек Steam на всех физических дисках (`libraryfolders.vdf`, `appmanifest_<id>.acf`) и манифестов Epic Games. Точное определение реальной папки игры и размера на диске (`SizeOnDisk`), исключая путаницу с папками кэша иконок.
- **Полный учет установленного ПО:** Единый список для классических программ Win32 (x64 / x86), приложений Магазина Windows (UWP / AppX) и Portable-программ.
- **4-этапный Мастер удаления остатков:** Глубокий аудит файловой системы (Program Files, AppData, Local, Roaming, ProgramData) и веток реестра (HKLM/HKCU/WOW6432Node). Иерархическая фильтрация и гарантированное удаление заблокированных файлов при перезагрузке (`MoveFileExW`).
- **Очистка кэша шейдеров GPU:** Очистка кэша шейдеров NVIDIA (`DXCache`, `GLCache`), AMD (`DxCache`), системного кэша DirectX и конвейеров Vulkan для освобождения диска и устранения микрофризов в играх.
- **Санитайзер переменных PATH:** Проверка системной и пользовательской переменной `PATH`, выявление несуществующих каталогов и дубликатов с исправлением в один клик.
- **Анализатор дубликатов VC++ и Java:** Автоматическая группировка библиотек Microsoft Visual C++ и сред Java для наглядного выявления и удаления устаревших версий.
- **Интеграция с Winget:** Поддержка Windows Package Manager (Winget CLI) для тихой деинсталляции и проверки доступных официальных обновлений.
- **Монитор установки (Install Tracker):** Создание побайтового снимка системы до и после инсталляции программы для гарантированного 100% чистого удаления в будущем.
- **Эргономичный нативный интерфейс:** Аппаратно-ускоренный интерфейс на `egui 0.30`, нейтральная графитовая темная и светлая темы, плавное масштабирование шрифтов и извлечение оригинальных иконок `.ico`.

### Горячие клавиши

| Хоткей | Действие |
|---|---|
| `F5` | Обновить список установленных программ |
| `Ctrl + F` | Переместить фокус в поле поиска |
| `Escape` | Очистить строку поиска |

### Сборка из исходников

#### Требования
- Windows 10 / 11 (64-бит)
- Rust 1.80+ (`stable-x86_64-pc-windows-msvc`)

```bash
git clone https://github.com/your-username/PurgeCraft.git
cd PurgeCraft
cargo build --release --bin purgecraft --manifest-path desktop_rust/Cargo.toml
```

Готовый автономный исполняемый файл будет доступен по пути:  
`desktop_rust/target/release/purgecraft.exe`

---

## License / Лицензия

Distributed under the **MIT License**. See [LICENSE](LICENSE) for more information.  
Распространяется под свободной лицензией **MIT**. Подробности в файле [LICENSE](LICENSE).
