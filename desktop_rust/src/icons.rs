#![allow(dead_code, non_snake_case, clippy::upper_case_acronyms)]

use std::path::Path;
use eframe::egui::ColorImage;

#[repr(C)]
struct ICONINFO {
    fIcon: i32,
    xHotspot: u32,
    yHotspot: u32,
    hbmMask: isize,
    hbmColor: isize,
}

#[repr(C)]
struct BITMAPINFOHEADER {
    biSize: u32,
    biWidth: i32,
    biHeight: i32,
    biPlanes: u16,
    biBitCount: u16,
    biCompression: u32,
    biSizeImage: u32,
    biXPelsPerMeter: i32,
    biYPelsPerMeter: i32,
    biClrUsed: u32,
    biClrImportant: u32,
}

#[repr(C)]
struct BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [u32; 1],
}

#[repr(C)]
struct SHFILEINFOW {
    hIcon: isize,
    iIcon: i32,
    dwAttributes: u32,
    szDisplayName: [u16; 260],
    szTypeName: [u16; 80],
}

const SHGFI_ICON: u32 = 0x000000100;
const SHGFI_LARGEICON: u32 = 0x000000000;
const SHGFI_SMALLICON: u32 = 0x000000001;

#[link(name = "shell32")]
extern "system" {
    fn ExtractIconExW(
        lpszFile: *const u16,
        nIconIndex: i32,
        phiconLarge: *mut isize,
        phiconSmall: *mut isize,
        nIcons: u32,
    ) -> u32;

    fn SHGetFileInfoW(
        pszPath: *const u16,
        dwFileAttributes: u32,
        psfi: *mut SHFILEINFOW,
        cbFileInfo: u32,
        uFlags: u32,
    ) -> usize;
}

#[link(name = "user32")]
extern "system" {
    fn DestroyIcon(hIcon: isize) -> i32;
    fn GetIconInfo(hIcon: isize, piconinfo: *mut ICONINFO) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn DeleteObject(hObject: isize) -> i32;
    fn GetDIBits(
        hdc: isize,
        hbm: isize,
        start: u32,
        cLines: u32,
        lpvBits: *mut u8,
        lpbmi: *mut BITMAPINFO,
        usage: u32,
    ) -> i32;
    fn CreateCompatibleDC(hdc: isize) -> isize;
    fn DeleteDC(hdc: isize) -> i32;
}

/// Parses `"C:\Path\App.exe,0"` or `"\"C:\Path\App.exe\""` into `(file_path, icon_index)`.
pub fn parse_icon_location(icon_raw_path: &str) -> (&str, i32) {
    let clean = icon_raw_path.trim().trim_matches('"');
    if let Some(idx_pos) = clean.rfind(',') {
        let (f, idx_str) = clean.split_at(idx_pos);
        let trimmed_f = f.trim().trim_matches('"');
        let idx_part = idx_str[1..].trim().trim_matches('"');
        if let Ok(idx) = idx_part.parse::<i32>() {
            return (trimmed_f, idx);
        }
        if Path::new(trimmed_f).exists() {
            return (trimmed_f, 0);
        }
    }
    (clean, 0)
}

pub fn extract_app_icon(icon_raw_path: &str) -> Option<ColorImage> {
    if icon_raw_path.is_empty() {
        return None;
    }

    let (file_path, icon_index) = parse_icon_location(icon_raw_path);

    if !Path::new(file_path).exists() {
        return None;
    }

    let wide_path: Vec<u16> = file_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut chosen_hicon: isize = 0;

    // SAFETY: All Win32 pointers point to live stack/Vec buffers with matching sizes,
    // `SHFILEINFOW`/`ICONINFO`/`BITMAPINFO` are C POD structs valid when zero-initialized,
    // and every acquired HICON, HBITMAP, and HDC handle is explicitly destroyed before returning.
    unsafe {
        let mut hicon_large: isize = 0;
        let mut hicon_small: isize = 0;
        let count = ExtractIconExW(
            wide_path.as_ptr(),
            icon_index,
            &mut hicon_large,
            &mut hicon_small,
            1,
        );

        if count > 0 {
            if hicon_large != 0 {
                chosen_hicon = hicon_large;
                if hicon_small != 0 {
                    DestroyIcon(hicon_small);
                }
            } else if hicon_small != 0 {
                chosen_hicon = hicon_small;
            }
        }

        // Fallback to Explorer SHGetFileInfoW if ExtractIconExW didn't yield an icon
        if chosen_hicon == 0 {
            let mut shfi: SHFILEINFOW = std::mem::zeroed();
            let res = SHGetFileInfoW(
                wide_path.as_ptr(),
                0,
                &mut shfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if res != 0 && shfi.hIcon != 0 {
                chosen_hicon = shfi.hIcon;
            }
        }

        if chosen_hicon == 0 {
            return None;
        }

        let mut icon_info: ICONINFO = std::mem::zeroed();
        if GetIconInfo(chosen_hicon, &mut icon_info) == 0 {
            DestroyIcon(chosen_hicon);
            return None;
        }

        let cleanup_icon_resources = |info: &ICONINFO, hicon: isize| {
            if info.hbmColor != 0 {
                DeleteObject(info.hbmColor);
            }
            if info.hbmMask != 0 {
                DeleteObject(info.hbmMask);
            }
            DestroyIcon(hicon);
        };

        let hbm_to_read = if icon_info.hbmColor != 0 { icon_info.hbmColor } else { icon_info.hbmMask };
        let hdc = CreateCompatibleDC(0);
        if hdc == 0 || hbm_to_read == 0 {
            if hdc != 0 {
                DeleteDC(hdc);
            }
            cleanup_icon_resources(&icon_info, chosen_hicon);
            return None;
        }

        let width = 32usize;
        let height = 32usize;
        let mut pixels = vec![0u8; width * height * 4];

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width as i32;
        bmi.bmiHeader.biHeight = -(height as i32); // Top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = 0; // BI_RGB

        let lines = GetDIBits(
            hdc,
            hbm_to_read,
            0,
            height as u32,
            pixels.as_mut_ptr(),
            &mut bmi,
            0,
        );

        DeleteDC(hdc);
        cleanup_icon_resources(&icon_info, chosen_hicon);

        if lines == 0 {
            return None;
        }

        // Convert BGRA to RGBA
        let (chunks, _) = pixels.as_chunks_mut::<4>();
        for chunk in chunks {
            chunk.swap(0, 2);
            // If alpha is 0 on a non-black pixel, fix alpha
            if chunk[3] == 0 && (chunk[0] > 0 || chunk[1] > 0 || chunk[2] > 0) {
                chunk[3] = 255;
            }
        }

        Some(ColorImage::from_rgba_unmultiplied([width, height], &pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_icon_location_formats() {
        assert_eq!(parse_icon_location(r#""C:\Program Files\App\app.exe",0"#), (r#"C:\Program Files\App\app.exe"#, 0));
        assert_eq!(parse_icon_location(r#"C:\App\app.exe, -101"#), (r#"C:\App\app.exe"#, -101));
        assert_eq!(parse_icon_location(r#""C:\Path, With Comma\app.exe""#), (r#"C:\Path, With Comma\app.exe"#, 0));
    }

    #[test]
    fn test_extract_app_icon_missing_or_empty() {
        assert!(extract_app_icon("").is_none());
        assert!(extract_app_icon(r"C:\non_existent_purgecraft_test_12345.exe,0").is_none());
    }
}
