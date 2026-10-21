//! Localization module for PurgeCraft (Russian / English)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Russian,
    English,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        if s.to_lowercase().contains("en") {
            Language::English
        } else {
            Language::Russian
        }
    }
}

pub fn t(lang: Language, ru: &'static str, en: &'static str) -> &'static str {
    match lang {
        Language::Russian => ru,
        Language::English => en,
    }
}

// Tabs
pub fn tab_uninstaller(lang: Language) -> &'static str {
    t(lang, "📦 Программы (Ctrl+1)", "📦 Programs (Ctrl+1)")
}

pub fn tab_treemap(lang: Language) -> &'static str {
    t(lang, "📊 Размер диска (Ctrl+2)", "📊 Disk Space (Ctrl+2)")
}

pub fn tab_startup(lang: Language) -> &'static str {
    t(lang, "⚡ Автозагрузка (Ctrl+3)", "⚡ Startup (Ctrl+3)")
}

// Menus
pub fn menu_file(lang: Language) -> &'static str {
    t(lang, "📁 Файл ▾", "📁 File ▾")
}

pub fn menu_tools(lang: Language) -> &'static str {
    t(lang, "🛠 Утилиты ▾", "🛠 Tools ▾")
}

pub fn menu_help(lang: Language) -> &'static str {
    t(lang, "❓ Справка ▾", "❓ Help ▾")
}

pub fn menu_refresh(lang: Language) -> &'static str {
    t(lang, "🔄 Обновить список (F5)", "🔄 Refresh List (F5)")
}

pub fn menu_settings(lang: Language) -> &'static str {
    t(lang, "⚙ Настройки...", "⚙ Settings...")
}

pub fn menu_exit(lang: Language) -> &'static str {
    t(lang, "❌ Выход", "❌ Exit")
}

pub fn menu_system_clean(lang: Language) -> &'static str {
    t(lang, "♻ Очистка системы Windows...", "♻ Windows System Clean...")
}

pub fn menu_dev_clean(lang: Language) -> &'static str {
    t(lang, "🛠 Кэши сред разработки (npm, cargo...)...", "🛠 Developer Caches (npm, cargo...)...")
}

pub fn menu_services(lang: Language) -> &'static str {
    t(lang, "⚙ Менеджер служб и зомби...", "⚙ Services & Zombie Processes...")
}

pub fn menu_path(lang: Language) -> &'static str {
    t(lang, "🗄 Санитайзер переменной PATH...", "🗄 PATH Sanitizer...")
}

pub fn menu_portable(lang: Language) -> &'static str {
    t(lang, "📦 Сканер Portable...", "📦 Portable Apps Scanner...")
}

pub fn menu_runtimes(lang: Language) -> &'static str {
    t(lang, "⚖ Анализатор дубликатов VC++/Java...", "⚖ VC++/Java Duplicate Analyzer...")
}

pub fn menu_winget(lang: Language) -> &'static str {
    t(lang, "🚀 Менеджер Winget...", "🚀 Winget Manager...")
}

pub fn menu_about(lang: Language) -> &'static str {
    t(lang, "ℹ О программе PurgeCraft", "ℹ About PurgeCraft")
}

pub fn menu_check_updates(lang: Language) -> &'static str {
    t(lang, "🔄 Проверить обновления...", "🔄 Check for Updates...")
}

// Uninstaller Actions
pub fn search_placeholder(lang: Language) -> &'static str {
    t(lang, "🔍 Поиск программы... (Ctrl+F, Esc для сброса)", "🔍 Search programs... (Ctrl+F, Esc to clear)")
}

pub fn btn_uninstall(lang: Language) -> &'static str {
    t(lang, "🗑 Деинсталляция (Del)", "🗑 Uninstall (Del)")
}

pub fn btn_force_remove(lang: Language) -> &'static str {
    t(lang, "⚡ Принудительное удаление", "⚡ Force Removal")
}

pub fn btn_install_tracker(lang: Language) -> &'static str {
    t(lang, "📸 Установка с мониторингом...", "📸 Install with Tracker...")
}

pub fn btn_batch_uninstall(lang: Language) -> &'static str {
    t(lang, "📦 Пакетное удаление...", "📦 Batch Uninstall...")
}

// Startup Actions
pub fn btn_add_startup(lang: Language) -> &'static str {
    t(lang, "➕ Добавить программу в автозапуск", "➕ Add Program to Startup")
}

pub fn btn_disable(lang: Language) -> &'static str {
    t(lang, "⏸ Отключить", "⏸ Disable")
}

pub fn btn_enable(lang: Language) -> &'static str {
    t(lang, "▶ Включить", "▶ Enable")
}

pub fn btn_delete(lang: Language) -> &'static str {
    t(lang, "🗑 Удалить", "🗑 Delete")
}

pub fn btn_jump_reg(lang: Language) -> &'static str {
    t(lang, "🧭 Перейти в реестре", "🧭 Jump in RegEdit")
}

// Columns
pub fn col_name(lang: Language) -> &'static str {
    t(lang, "Имя программы", "Application Name")
}

pub fn col_size(lang: Language) -> &'static str {
    t(lang, "Размер", "Size")
}

pub fn col_date(lang: Language) -> &'static str {
    t(lang, "Дата установки", "Install Date")
}

pub fn col_version(lang: Language) -> &'static str {
    t(lang, "Версия", "Version")
}

pub fn col_publisher(lang: Language) -> &'static str {
    t(lang, "Издатель", "Publisher")
}

// Categories & Filters
pub fn filter_all(lang: Language) -> &'static str {
    t(lang, "Все", "All")
}

pub fn category_user(lang: Language) -> &'static str {
    t(lang, "Программы", "Programs")
}

pub fn category_system(lang: Language) -> &'static str {
    t(lang, "Системные", "System")
}

pub fn filter_all_programs(lang: Language) -> &'static str {
    t(lang, "Все программы", "All Programs")
}

pub fn filter_win32(lang: Language) -> &'static str {
    t(lang, "Классические Win32", "Classic Win32")
}

pub fn filter_uwp(lang: Language) -> &'static str {
    t(lang, "Приложения Windows Store", "Windows Store Apps")
}

pub fn filter_system(lang: Language) -> &'static str {
    t(lang, "Системные компоненты", "System Components")
}

pub fn filter_hidden(lang: Language) -> &'static str {
    t(lang, "Скрытые программы", "Hidden Programs")
}

pub fn filter_menu_title(lang: Language) -> &'static str {
    t(lang, "Фильтры ▾", "Filters ▾")
}

pub fn filter_reset(lang: Language) -> &'static str {
    t(lang, "✕ Сбросить фильтры", "✕ Reset filters")
}

pub fn filter_app_type_header(lang: Language) -> &'static str {
    t(lang, "Тип приложения:", "Application type:")
}

pub fn filter_portable_label(lang: Language) -> &'static str {
    t(lang, "📦 Portable", "📦 Portable")
}

pub fn filter_store_label(lang: Language) -> &'static str {
    t(lang, "🏪 Магазин Windows", "🏪 Windows Store")
}

pub fn filter_arch_header(lang: Language) -> &'static str {
    t(lang, "Архитектура:", "Architecture:")
}

pub fn filter_arch_64(lang: Language) -> &'static str {
    t(lang, "⚙ 64-бит", "⚙ 64-bit")
}

pub fn filter_arch_32(lang: Language) -> &'static str {
    t(lang, "⚙ 32-бит", "⚙ 32-bit")
}

pub fn filter_drive_header(lang: Language) -> &'static str {
    t(lang, "Диск установки:", "Install drive:")
}

pub fn filter_all_drives(lang: Language) -> &'static str {
    t(lang, "💾 Все диски", "💾 All drives")
}

pub fn filter_drive_item(lang: Language) -> &'static str {
    t(lang, "Диск", "Drive")
}

pub fn filter_apps_suffix(lang: Language) -> &'static str {
    t(lang, "прогр.", "apps")
}

pub fn selected_count(lang: Language) -> &'static str {
    t(lang, "Выбрано", "Selected")
}

pub fn view_treemap_btn(lang: Language) -> &'static str {
    t(lang, "📊 Treemap", "📊 Treemap")
}

pub fn view_treemap_tip(lang: Language) -> &'static str {
    t(lang, "Визуальная карта дискового пространства и крупнейших программ", "Visual map of disk space usage and largest programs")
}

pub fn view_list_btn(lang: Language) -> &'static str {
    t(lang, "📋 Список", "📋 List")
}

pub fn view_list_tip(lang: Language) -> &'static str {
    t(lang, "Табличный список установленных программ", "Table list of installed programs")
}

pub fn not_found_title(lang: Language) -> &'static str {
    t(lang, "Ничего не найдено", "Nothing found")
}

pub fn not_found_hint(lang: Language) -> &'static str {
    t(lang, "Попробуйте сбросить поисковый фильтр (Esc) или переключить категорию.", "Try clearing the search filter (Esc) or changing category.")
}

pub fn sidebar_actions_header(lang: Language) -> &'static str {
    t(lang, "Действия", "Actions")
}

pub fn sidebar_remove_entry(lang: Language) -> &'static str {
    t(lang, "❌ Удалить запись из списка", "❌ Remove entry from list")
}

pub fn sidebar_remove_entry_tip(lang: Language) -> &'static str {
    t(lang, "Удалить поврежденную запись из реестра Windows без запуска деинсталлятора", "Remove broken entry from Windows registry without running uninstaller")
}

pub fn sidebar_open_folder(lang: Language) -> &'static str {
    t(lang, "📁 Открыть папку программы", "📁 Open program folder")
}

pub fn sidebar_open_reg(lang: Language) -> &'static str {
    t(lang, "🗄 Ключ в реестре Windows", "🗄 Windows registry key")
}

pub fn sidebar_search_google(lang: Language) -> &'static str {
    t(lang, "🌐 Найти информацию в Google", "🌐 Search Google for info")
}

pub fn sidebar_select_hint(lang: Language) -> &'static str {
    t(lang, "Выберите программу из списка для просмотра сведений и действий.", "Select a program from the list to view details and actions.")
}

// Startup Strings
pub fn startup_filter_all(lang: Language) -> &'static str {
    t(lang, "Все", "All")
}

pub fn startup_filter_registry(lang: Language) -> &'static str {
    t(lang, "🗄 Реестр", "🗄 Registry")
}

pub fn startup_filter_startmenu(lang: Language) -> &'static str {
    t(lang, "📁 Меню Пуск", "📁 Start Menu")
}

pub fn startup_filter_taskscheduler(lang: Language) -> &'static str {
    t(lang, "⏰ Планировщик", "⏰ Task Scheduler")
}

pub fn startup_add_btn(lang: Language) -> &'static str {
    t(lang, "➕ Добавить программу...", "➕ Add program...")
}

pub fn startup_add_btn_tip(lang: Language) -> &'static str {
    t(lang, "Добавить новое приложение в автозапуск Windows", "Add a new application to Windows startup")
}

pub fn startup_col_state(lang: Language) -> &'static str {
    t(lang, "Состояние", "State")
}

pub fn startup_col_name(lang: Language) -> &'static str {
    t(lang, "Имя", "Name")
}

pub fn startup_col_impact(lang: Language) -> &'static str {
    t(lang, "Влияние", "Impact")
}

pub fn startup_col_source(lang: Language) -> &'static str {
    t(lang, "Источник", "Source")
}

pub fn startup_col_command(lang: Language) -> &'static str {
    t(lang, "Команда / Расположение", "Command / Location")
}

pub fn startup_col_action(lang: Language) -> &'static str {
    t(lang, "Действие", "Action")
}

pub fn startup_status_on(lang: Language) -> &'static str {
    t(lang, "✔ Вкл", "✔ On")
}

pub fn startup_status_off(lang: Language) -> &'static str {
    t(lang, "○ Откл", "○ Off")
}

pub fn startup_impact_high(lang: Language) -> &'static str {
    t(lang, "⚡ Высокое", "⚡ High")
}

pub fn startup_impact_medium(lang: Language) -> &'static str {
    t(lang, "🟡 Среднее", "🟡 Medium")
}

pub fn startup_impact_low(lang: Language) -> &'static str {
    t(lang, "🟢 Низкое", "🟢 Low")
}

pub fn startup_impact_none(lang: Language) -> &'static str {
    t(lang, "⚪ Не измерено", "⚪ Not measured")
}

