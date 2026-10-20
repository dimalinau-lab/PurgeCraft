use std::fs::{self, File};
use std::io::{Read, Write};
use std::process::Command;
use std::sync::mpsc::Sender;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GithubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub body: Option<String>,
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UpdateInfo {
    pub version: String,
    pub tag_name: String,
    pub html_url: String,
    pub release_notes: String,
    pub download_url: String,
    pub asset_name: String,
    pub file_size: u64,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available(UpdateInfo),
    Downloading { percent: f32, downloaded: u64, total: u64 },
    ReadyToRestart,
    Error(String),
}

/// Cleanup leftover `.old` executable from a previous update
pub fn cleanup_previous_update() {
    if let Ok(exe_path) = std::env::current_exe() {
        let old_exe = exe_path.with_extension("exe.old");
        if old_exe.exists() {
            let _ = fs::remove_file(old_exe);
        }
    }
}

/// Compare SemVer-like version strings ("1.0.0" vs "1.0.1")
pub fn is_version_newer(current: &str, latest: &str) -> bool {
    let cur = current.trim_start_matches('v').trim();
    let lat = latest.trim_start_matches('v').trim();

    let cur_parts: Vec<u32> = cur.split('.').filter_map(|s| s.split('-').next().unwrap_or(s).parse().ok()).collect();
    let lat_parts: Vec<u32> = lat.split('.').filter_map(|s| s.split('-').next().unwrap_or(s).parse().ok()).collect();

    let max_len = std::cmp::max(cur_parts.len(), lat_parts.len());
    for i in 0..max_len {
        let c = cur_parts.get(i).copied().unwrap_or(0);
        let l = lat_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// Checks GitHub for the latest release in a background thread
pub fn check_for_updates_async(repo: String, current_version: String, tx: Sender<UpdateStatus>) {
    std::thread::spawn(move || {
        let _ = tx.send(UpdateStatus::Checking);
        match check_for_updates_sync(&repo, &current_version) {
            Ok(Some(info)) => {
                let _ = tx.send(UpdateStatus::Available(info));
            }
            Ok(None) => {
                let _ = tx.send(UpdateStatus::UpToDate);
            }
            Err(e) => {
                let _ = tx.send(UpdateStatus::Error(e));
            }
        }
    });
}

/// Synchronous check against GitHub Releases API
pub fn check_for_updates_sync(repo: &str, current_version: &str) -> Result<Option<UpdateInfo>, String> {
    let clean_repo = repo.trim().trim_matches('/');
    if clean_repo.is_empty() {
        return Err("Репозиторий GitHub не указан".to_string());
    }

    let url = format!("https://api.github.com/repos/{}/releases/latest", clean_repo);

    let resp: GithubRelease = ureq::get(&url)
        .set("User-Agent", &format!("PurgeCraft/{}", current_version))
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| format!("Ошибка сети при обращении к GitHub: {}", e))?
        .into_json()
        .map_err(|e| format!("Не удалось разобрать ответ GitHub: {}", e))?;

    let latest_tag = resp.tag_name.trim_start_matches('v').to_string();
    if !is_version_newer(current_version, &latest_tag) {
        return Ok(None);
    }

    // Find the Windows .exe asset
    let exe_asset = resp.assets.iter().find(|a| {
        let lower = a.name.to_lowercase();
        lower.ends_with(".exe") && !lower.contains("setup")
    }).or_else(|| {
        resp.assets.iter().find(|a| a.name.to_lowercase().ends_with(".exe"))
    });

    if let Some(asset) = exe_asset {
        Ok(Some(UpdateInfo {
            version: latest_tag,
            tag_name: resp.tag_name,
            html_url: resp.html_url,
            release_notes: resp.body.unwrap_or_else(|| "Описание релиза отсутствует".to_string()),
            download_url: asset.browser_download_url.clone(),
            asset_name: asset.name.clone(),
            file_size: asset.size,
        }))
    } else {
        Err(format!("В релизе {} не найден файл .exe для обновления", resp.tag_name))
    }
}

/// Downloads and performs atomic Windows in-place executable replacement
pub fn download_and_apply_update_async(download_url: String, tx: Sender<UpdateStatus>) {
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let current_exe = std::env::current_exe().map_err(|e| format!("Не удалось определить путь к exe: {}", e))?;
            let temp_exe = current_exe.with_extension("download");
            let old_exe = current_exe.with_extension("exe.old");

            let resp = ureq::get(&download_url)
                .set("User-Agent", "PurgeCraft-Updater")
                .timeout(std::time::Duration::from_secs(120))
                .call()
                .map_err(|e| format!("Ошибка загрузки обновления: {}", e))?;

            let total_size = resp.header("Content-Length")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);

            let mut reader = resp.into_reader();
            let mut file = File::create(&temp_exe).map_err(|e| format!("Не удалось создать временный файл: {}", e))?;
            let mut buffer = [0u8; 16384];
            let mut downloaded: u64 = 0;
            let mut last_percent: f32 = 0.0;

            loop {
                let n = reader.read(&mut buffer).map_err(|e| format!("Ошибка чтения потока: {}", e))?;
                if n == 0 {
                    break;
                }
                file.write_all(&buffer[..n]).map_err(|e| format!("Ошибка записи файла: {}", e))?;
                downloaded += n as u64;

                let percent = if total_size > 0 {
                    downloaded as f32 / total_size as f32
                } else {
                    0.0
                };

                if (percent - last_percent).abs() >= 0.02 || downloaded == total_size {
                    last_percent = percent;
                    let _ = tx.send(UpdateStatus::Downloading {
                        percent,
                        downloaded,
                        total: total_size,
                    });
                }
            }
            file.flush().map_err(|e| format!("Ошибка сброса буфера на диск: {}", e))?;
            drop(file);

            // Windows atomic swap via rename:
            // 1. Delete previous .old if exists
            if old_exe.exists() {
                let _ = fs::remove_file(&old_exe);
            }

            // 2. Rename running exe to exe.old
            fs::rename(&current_exe, &old_exe)
                .map_err(|e| format!("Не удалось переименовать текущий exe: {}", e))?;

            // 3. Rename downloaded temp file to current exe
            if let Err(e) = fs::rename(&temp_exe, &current_exe) {
                // Rollback if renaming new file failed
                let _ = fs::rename(&old_exe, &current_exe);
                return Err(format!("Ошибка установки нового exe (откат выполнен): {}", e));
            }

            Ok(())
        })();

        match result {
            Ok(()) => {
                let _ = tx.send(UpdateStatus::ReadyToRestart);
            }
            Err(e) => {
                let _ = tx.send(UpdateStatus::Error(e));
            }
        }
    });
}

/// Spawns the updated executable and exits the current process
pub fn restart_app() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("Не удалось запустить обновленное приложение: {}", e))?;
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_newer() {
        assert!(is_version_newer("1.0.0", "1.0.1"));
        assert!(is_version_newer("1.0.0", "1.1.0"));
        assert!(is_version_newer("1.0.0", "2.0.0"));
        assert!(is_version_newer("v1.0.0", "v1.0.1"));
        assert!(is_version_newer("1.0.0", "v1.0.1-beta"));
        assert!(!is_version_newer("1.0.1", "1.0.0"));
        assert!(!is_version_newer("1.0.0", "1.0.0"));
        assert!(!is_version_newer("1.2.0", "1.1.9"));
    }
}
