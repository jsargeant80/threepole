use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use tauri::{
    async_runtime::{self, JoinHandle},
    AppHandle, Manager,
};
use widestring::Utf16String;
use windows::Win32::{
    Foundation::{HWND, MAX_PATH},
    System::{
        ProcessStatus::K32GetModuleFileNameExW,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
    },
    UI::WindowsAndMessaging::{
        GetTopWindow, GetWindow, GetWindowTextW, GetWindowThreadProcessId, GW_HWNDNEXT,
        IsWindowVisible,
    },
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyTrack {
    pub title: String,
    pub artist: String,
}

#[derive(Default)]
pub struct SpotifyPoller {
    task_handle: Option<JoinHandle<()>>,
}

impl SpotifyPoller {
    pub fn start(&mut self, handle: AppHandle) {
        if let Some(h) = &self.task_handle {
            h.abort();
        }
        self.task_handle = Some(async_runtime::spawn(poll_loop(handle)));
    }

    pub fn stop(&mut self) {
        if let Some(h) = &self.task_handle {
            h.abort();
        }
        self.task_handle = None;
    }
}

async fn poll_loop(handle: AppHandle) {
    let mut last_track: Option<String> = None;

    loop {
        let track = find_spotify_track();

        let key = track.as_ref().map(|t| format!("{}-{}", t.artist, t.title));

        if key != last_track {
            last_track = key;
            if let Some(overlay) = handle.get_window("overlay") {
                let _ = overlay.emit("spotify_track", &track);
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

fn find_spotify_track() -> Option<SpotifyTrack> {
    let mut hwnd = unsafe { GetTopWindow(HWND(0)) };

    loop {
        if hwnd.0 == 0 {
            break;
        }

        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            if let Some(exec) = get_window_exec(hwnd) {
                if exec.eq_ignore_ascii_case("spotify.exe") {
                    if let Some(title) = get_window_title(hwnd) {
                        if let Some(track) = parse_title(&title) {
                            return Some(track);
                        }
                    }
                }
            }
        }

        hwnd = unsafe { GetWindow(hwnd, GW_HWNDNEXT) };
    }

    None
}

fn parse_title(title: &str) -> Option<SpotifyTrack> {
    // Spotify sets the window title to "Artist - Song" while playing
    // When idle/paused: "Spotify", "Spotify Premium", "Spotify Free", etc.
    if title.is_empty()
        || title == "Spotify"
        || title.starts_with("Spotify Premium")
        || title.starts_with("Spotify Free")
        || title == "Advertisement"
    {
        return None;
    }

    title.find(" - ").map(|i| SpotifyTrack {
        artist: title[..i].to_string(),
        title: title[i + 3..].to_string(),
    })
}

fn get_window_title(hwnd: HWND) -> Option<String> {
    let mut buf: [u16; 512] = [0; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if len <= 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

fn get_window_exec(hwnd: HWND) -> Option<String> {
    if hwnd.0 == 0 {
        return None;
    }

    let mut process_id = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };

    if process_id == 0 {
        return None;
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, false, process_id) }.ok()?;

    let mut buf: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
    unsafe { K32GetModuleFileNameExW(handle, None, &mut buf) };

    let mut s = Utf16String::from_slice_lossy(&buf).to_string();
    s.retain(|c| c != '\0');

    PathBuf::from(s)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}
