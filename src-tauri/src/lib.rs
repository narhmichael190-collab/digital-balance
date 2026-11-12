#![allow(non_snake_case)]

use chrono::{DateTime, Local, Utc};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, HashSet}, fs, path::PathBuf, process::Command, sync::{Arc, Mutex}, thread, time::Duration};
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt as AutoStartManagerExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AppMode { Casual, Smart, #[serde(rename = "Deep Focus")] DeepFocus }
impl Default for AppMode { fn default() -> Self { Self::Smart } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Category { Productive, Distracting, Communication, Media, Neutral }
impl Default for Category { fn default() -> Self { Self::Neutral } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LimitAction { Warn, Close, Block }
impl Default for LimitAction { fn default() -> Self { Self::Block } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitScope { Daily, UntilDate }
impl Default for LimitScope { fn default() -> Self { Self::Daily } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLimit {
    pub dailyMinutes: u32,
    #[serde(default)] pub action: LimitAction,
    #[serde(default = "default_true")] pub enabled: bool,
    #[serde(default)] pub scope: LimitScope,
    #[serde(default)] pub startedAt: Option<String>,
    #[serde(default)] pub expiresAt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub dailyDistractionLimitMinutes: u32,
    pub dailyFocusTargetMinutes: u32,
    pub idleThresholdSeconds: u64,
    pub mode: AppMode,
    pub notificationsEnabled: bool,
    pub autostartEnabled: bool,
    #[serde(default = "default_true")] pub closeToTray: bool,
    #[serde(default = "default_true")] pub motivationalNudges: bool,
    #[serde(default = "default_display_name")] pub displayName: String,
    pub categoryOverrides: HashMap<String, Category>,
    #[serde(default)] pub appLimits: HashMap<String, AppLimit>,
    #[serde(default = "default_theme")] pub theme: String,
    #[serde(default)] pub purchasedThemes: Vec<String>,
    #[serde(default)] pub coins: u32,
    #[serde(default)] pub ownedRewards: Vec<String>,
    #[serde(default = "default_frame")] pub equippedFrame: String,
    #[serde(default = "default_effect")] pub equippedEffect: String,
    #[serde(default)] pub deepFocusBlockedApps: Vec<String>,
    #[serde(default = "default_true")] pub deepFocusAutoBlock: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dailyDistractionLimitMinutes: 90,
            dailyFocusTargetMinutes: 60,
            idleThresholdSeconds: 120,
            mode: AppMode::Smart,
            notificationsEnabled: true,
            autostartEnabled: false,
            closeToTray: true,
            motivationalNudges: true,
            displayName: "Digital Balance".into(),
            categoryOverrides: HashMap::new(),
            appLimits: HashMap::new(),
            theme: "theme:obsidian".into(),
            purchasedThemes: vec!["theme:obsidian".into()],
            coins: 0,
            ownedRewards: free_theme_ids(),
            equippedFrame: "frame:classic".into(),
            equippedEffect: "effect:clean".into(),
            deepFocusBlockedApps: Vec::new(),
            deepFocusAutoBlock: true,
        }
    }
}

fn default_true() -> bool { true }
fn default_display_name() -> String { "Digital Balance".into() }
fn default_theme() -> String { "theme:obsidian".into() }
fn default_frame() -> String { "frame:classic".into() }
fn default_effect() -> String { "effect:clean".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord { pub app_name: String, pub exe: String, pub category: Category, pub started_at: String, pub ended_at: String, pub duration_secs: u64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSession { pub minutes: u32, pub points: u32, pub coins: u32, pub completed_at: String, pub mode: AppMode }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerData {
    pub sessions: Vec<SessionRecord>,
    pub focus_sessions: Vec<FocusSession>,
    pub current_app: Option<String>,
    pub current_exe: Option<String>,
    pub current_category: Option<Category>,
    pub current_session_secs: u64,
    pub tracking_paused: bool,
    pub settings: AppSettings,
}
impl Default for TrackerData {
    fn default() -> Self { Self { sessions: Vec::new(), focus_sessions: Vec::new(), current_app: None, current_exe: None, current_category: None, current_session_secs: 0, tracking_paused: false, settings: AppSettings::default() } }
}

#[derive(Debug, Clone)]
struct ActiveSession { exe: String, app_name: String, category: Category, pid: u32, started: DateTime<Utc> }

#[derive(Debug)]
struct Tracker { data: TrackerData, path: PathBuf, active: Option<ActiveSession>, focus_active: bool, focus_mode: Option<AppMode>, focus_started: Option<DateTime<Utc>>, focus_apps: HashSet<String>, casual_focus_started: HashMap<String, DateTime<Utc>>, closed_today: HashSet<String>, warn_pending: HashSet<String>, warn_reopen_counts: HashMap<String, u8>, warning_active: Option<(String, u32)>, closed_date: String }

#[derive(Clone)]
pub struct AppState(pub Arc<Mutex<Tracker>>);
static GLOBAL_TRACKER: OnceCell<Arc<Mutex<Tracker>>> = OnceCell::new();
static GLOBAL_APP: OnceCell<AppHandle> = OnceCell::new();

fn local_day() -> String { Local::now().date_naive().to_string() }

fn safe_exe(exe: &str) -> bool {
    let lower = exe.to_ascii_lowercase();
    if exe.is_empty() || exe.len() > 160 { return false; }
    if !exe.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')) { return false; }
    !matches!(lower.as_str(), "system" | "smss.exe" | "csrss.exe" | "wininit.exe" | "winlogon.exe" | "services.exe" | "lsass.exe" | "svchost.exe" | "dwm.exe" | "explorer.exe" | "digital-balance.exe")
}

fn free_theme_ids() -> Vec<String> {
    ["theme:obsidian", "theme:aurora", "theme:ember", "theme:forest", "theme:ocean", "theme:violet", "theme:rose", "theme:sunset"]
        .into_iter().map(str::to_string).collect()
}

fn reward_price(id: &str) -> Option<u32> {
    if let Some(raw) = id.strip_prefix("theme:theme-") {
        if let Ok(index) = raw.parse::<u32>() {
            if (1..=100).contains(&index) {
                let price = if index <= 8 {
                    0
                } else if index >= 93 {
                    3000 + (index - 93) * 260
                } else {
                    180 + ((index - 9) / 8) * 170 + ((index - 9) % 8) * 35
                };
                return Some(price);
            }
        }
    }
    Some(match id {
        "theme:obsidian" | "theme:aurora" | "theme:ember" | "theme:forest" | "theme:ocean" | "theme:violet" | "theme:rose" | "theme:sunset" => 0,
        "frame:neon" => 300,
        "frame:gold" => 450,
        "frame:prism" => 900,
        "effect:orbit" => 400,
        "effect:pulse" => 350,
        "effect:matrix" => 750,
        "effect:nebula" => 1200,
        "effect:prism" => 1500,
        "effect:golden" => 1800,
        _ => return None,
    })
}


impl Tracker {
    fn load(path: PathBuf) -> Self {
        let mut data = fs::read_to_string(&path).ok().and_then(|raw| serde_json::from_str::<TrackerData>(&raw).ok()).unwrap_or_default();
        if data.settings.purchasedThemes.is_empty() { data.settings.purchasedThemes = free_theme_ids(); }
        let mut owned = if data.settings.ownedRewards.is_empty() { data.settings.purchasedThemes.clone() } else { data.settings.ownedRewards.clone() };
        for id in free_theme_ids() { if !owned.contains(&id) { owned.push(id); } }
        data.settings.ownedRewards = owned;
        if data.settings.displayName.trim().is_empty() { data.settings.displayName = "Digital Balance".into(); }
        if data.settings.theme.is_empty() { data.settings.theme = "theme:obsidian".into(); }
        Self { data, path, active: None, focus_active: false, focus_mode: None, focus_started: None, focus_apps: HashSet::new(), casual_focus_started: HashMap::new(), closed_today: HashSet::new(), warn_pending: HashSet::new(), warn_reopen_counts: HashMap::new(), warning_active: None, closed_date: local_day() }
    }
    fn persist(&self) {
        if let Some(parent) = self.path.parent() { let _ = fs::create_dir_all(parent); }
        if let Ok(raw) = serde_json::to_string_pretty(&self.data) { let _ = fs::write(&self.path, raw); }
    }
    fn normalize_day(&mut self) { let now = local_day(); if now != self.closed_date { self.closed_date = now; self.closed_today.clear(); self.warn_pending.clear(); self.warn_reopen_counts.clear(); self.warning_active = None; } }
    fn category_for(&self, exe: &str) -> Category {
        self.data.settings.categoryOverrides.get(exe).cloned().unwrap_or_else(|| auto_category(exe))
    }
    fn finish_active(&mut self) {
        if let Some(active) = self.active.take() {
            let seconds = (Utc::now() - active.started).num_seconds().max(0) as u64;
            self.data.current_session_secs = 0;
            if seconds >= 1 {
                self.data.sessions.push(SessionRecord { app_name: active.app_name, exe: active.exe, category: active.category, started_at: active.started.to_rfc3339(), ended_at: Utc::now().to_rfc3339(), duration_secs: seconds });
                if self.data.sessions.len() > 4000 { let drain = self.data.sessions.len() - 4000; self.data.sessions.drain(0..drain); }
                self.persist();
            }
        }
    }
    fn usage_today(&self, exe: &str) -> u64 {
        let day = Local::now().date_naive();
        let mut total = 0u64;
        for session in self.data.sessions.iter().filter(|s| s.exe.eq_ignore_ascii_case(exe)) {
            if let (Ok(start), Ok(end)) = (DateTime::parse_from_rfc3339(&session.started_at), DateTime::parse_from_rfc3339(&session.ended_at)) {
                if start.with_timezone(&Local).date_naive() == day { total = total.saturating_add((end - start).num_seconds().max(0) as u64); }
            }
        }
        if let Some(active) = &self.active { if active.exe.eq_ignore_ascii_case(exe) { total = total.saturating_add((Utc::now() - active.started).num_seconds().max(0) as u64); } }
        total
    }

    fn usage_since(&self, exe: &str, since: DateTime<Utc>) -> u64 {
        let now = Utc::now();
        let mut total = 0u64;
        for session in self.data.sessions.iter().filter(|s| s.exe.eq_ignore_ascii_case(exe)) {
            if let (Ok(start), Ok(end)) = (DateTime::parse_from_rfc3339(&session.started_at), DateTime::parse_from_rfc3339(&session.ended_at)) {
                let start = start.with_timezone(&Utc);
                let end = end.with_timezone(&Utc);
                let effective_start = if start < since { since } else { start };
                let effective_end = if end > now { now } else { end };
                if effective_end > effective_start { total = total.saturating_add((effective_end - effective_start).num_seconds().max(0) as u64); }
            }
        }
        if let Some(active) = &self.active {
            if active.exe.eq_ignore_ascii_case(exe) {
                let effective_start = if active.started < since { since } else { active.started };
                if now > effective_start { total = total.saturating_add((now - effective_start).num_seconds().max(0) as u64); }
            }
        }
        total
    }

    fn usage_for_rule(&self, exe: &str, rule: &AppLimit) -> u64 {
        match rule.scope {
            LimitScope::Daily => self.usage_today(exe),
            LimitScope::UntilDate => {
                let since = rule.startedAt.as_deref().and_then(|raw| DateTime::parse_from_rfc3339(raw).ok()).map(|dt| dt.with_timezone(&Utc)).unwrap_or_else(Utc::now);
                self.usage_since(exe, since)
            }
        }
    }

    fn refresh_foreground(&mut self) {
        if self.data.tracking_paused { return; }
        if let Some(info) = current_identity() {
            if self.active.as_ref().map(|x| x.pid) != Some(info.pid) {
                self.finish_active();
                let category = self.category_for(&info.exe);
                self.active = Some(ActiveSession { exe: info.exe.clone(), app_name: info.app_name.clone(), category: category.clone(), pid: info.pid, started: Utc::now() });
                self.data.current_app = Some(info.app_name);
                self.data.current_exe = Some(info.exe.clone());
                self.data.current_category = Some(category);
                self.data.current_session_secs = 0;
                self.persist();
            }
        }
    }
    fn current_data(&mut self) -> TrackerData {
        self.normalize_day();
        self.refresh_foreground();
        self.enforce_active();
        let mut out = self.data.clone();
        if let Some(active) = &self.active { out.current_session_secs = (Utc::now() - active.started).num_seconds().max(0) as u64; }
        else { out.current_session_secs = 0; }
        out.tracking_paused = self.data.tracking_paused;
        out
    }
    fn enforce_active(&mut self) {
        self.normalize_day();
        if self.data.tracking_paused { return; }

        if let Some((_, warning_pid)) = self.warning_active.clone() {
            if !process_exists(warning_pid) {
                self.warning_active = None;
                hide_limit_overlay();
            }
        }

        let Some(active) = self.active.clone() else { return; };
        let active_key = active.exe.to_ascii_lowercase();

        if self.focus_active && self.focus_apps.contains(&active_key) {
            match self.focus_mode.clone().unwrap_or(AppMode::Smart) {
                AppMode::DeepFocus | AppMode::Smart => {
                    self.close_pid(active.pid, &active.exe, "Focus protection");
                    return;
                }
                AppMode::Casual => {
                    let started = self.casual_focus_started.entry(active_key.clone()).or_insert_with(Utc::now);
                    let grace = (Utc::now() - *started).num_seconds().max(0) as u64;
                    if grace >= 10 * 60 {
                        if let Some(handle) = GLOBAL_APP.get() { let _ = handle.emit("focus-grace-expired", serde_json::json!({"app": active.app_name, "limitMinutes": 10})); }
                        self.close_pid(active.pid, &active.exe, "Casual Focus grace period ended");
                        return;
                    }
                }
            }
        }

        let Some(rule) = self.data.settings.appLimits.get(&active.exe).cloned() else { return; };
        if !rule.enabled { return; }
        if rule.scope == LimitScope::UntilDate {
            if let Some(expires) = rule.expiresAt.as_deref().and_then(|raw| DateTime::parse_from_rfc3339(raw).ok()) {
                if Utc::now() >= expires.with_timezone(&Utc) { return; }
            }
        }
        let used_seconds = self.usage_for_rule(&active.exe, &rule);
        let limit_seconds = u64::from(rule.dailyMinutes).saturating_mul(60);
        if used_seconds < limit_seconds { return; }
        match rule.action {
            LimitAction::Warn => {
                let key = active_key.clone();
                if self.warning_active.as_ref().map(|(_, pid)| *pid) == Some(active.pid) { return; }
                if self.warn_pending.remove(&key) {
                    let count = self.warn_reopen_counts.entry(key.clone()).or_insert(0);
                    *count = count.saturating_add(1);
                    if *count >= 3 {
                        if let Some(current) = self.data.settings.appLimits.get_mut(&active.exe) { current.action = LimitAction::Block; }
                        self.persist();
                        self.close_pid(active.pid, &active.exe, "Three warning reopen attempts reached — protection switched to Block");
                        hide_limit_overlay();
                        return;
                    }
                    self.warning_active = Some((key.clone(), active.pid));
                    show_limit_overlay(&active.exe, &active.app_name, active.pid, *count);
                    self.emit_limit(active.exe.clone(), active.app_name.clone(), "Warning — close this app to continue".into(), used_seconds, limit_seconds);
                } else {
                    self.warn_pending.insert(key);
                    self.close_pid(active.pid, &active.exe, "Limit reached — app closed before warning");
                }
            }
            LimitAction::Close => {
                if !self.closed_today.contains(&active_key) {
                    self.closed_today.insert(active_key);
                    self.close_pid(active.pid, &active.exe, "Limit reached — app closed");
                }
            }
            LimitAction::Block => {
                self.close_pid(active.pid, &active.exe, "Limit reached — blocked until the rule resets");
            }
        }
    }
    fn emit_limit(&self, exe: String, app: String, reason: String, used: u64, limit: u64) {
        if let Some(handle) = GLOBAL_APP.get() { let _ = handle.emit("app-limit-hit", serde_json::json!({"exe":exe,"app":app,"reason":reason,"usedMinutes":used/60,"limitMinutes":limit/60})); }
    }
    fn close_pid(&mut self, pid: u32, exe: &str, reason: &str) {
        if !safe_exe(exe) { return; }
        let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F", "/T"]).status();
        let (used, limit) = self.data.settings.appLimits.get(exe).map(|rule| (self.usage_for_rule(exe, rule), u64::from(rule.dailyMinutes).saturating_mul(60))).unwrap_or((0,0));
        self.emit_limit(exe.to_string(), exe.to_string(), reason.to_string(), used, limit);
        self.finish_active();
        self.active = None;
        self.data.current_app = None; self.data.current_exe = None; self.data.current_category = None; self.data.current_session_secs = 0;
    }
}

fn encode_url(input: &str) -> String { input.bytes().map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-'|b'_'|b'.') { (b as char).to_string() } else { format!("%{:02X}", b) }).collect() }

fn show_limit_overlay(exe: &str, app_name: &str, pid: u32, reopen: u8) {
    let Some(app) = GLOBAL_APP.get() else { return; };
    let _ = app.get_webview_window("limit-warning").map(|w| w.close());
    let url = format!("overlay.html?type=limit&app={}&exe={}&pid={}&reopen={}", encode_url(app_name), encode_url(exe), pid, reopen);
    let _ = WebviewWindowBuilder::new(app, "limit-warning", WebviewUrl::App(url.into()))
        .decorations(false).resizable(false).always_on_top(true).skip_taskbar(true).focused(true).fullscreen(true).build();
}

fn hide_limit_overlay() {
    if let Some(app) = GLOBAL_APP.get() { let _ = app.get_webview_window("limit-warning").map(|w| w.close()); }
}

fn process_exists(pid: u32) -> bool {
    #[cfg(windows)] {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        unsafe {
            match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Ok(handle) => { let _ = CloseHandle(handle); true },
                Err(_) => false,
            }
        }
    }
    #[cfg(not(windows))] { let _ = pid; false }
}

#[derive(Clone)]
struct ForegroundInfo { exe: String, app_name: String, pid: u32 }
fn auto_category(exe: &str) -> Category {
    let x = exe.to_ascii_lowercase();
    if ["code.exe","devenv.exe","idea64.exe","pycharm64.exe","notepad.exe","excel.exe","winword.exe","powershell.exe","cmd.exe"].iter().any(|v| x == *v) { return Category::Productive; }
    if ["discord.exe","whatsapp.exe","telegram.exe","slack.exe"].iter().any(|v| x == *v) { return Category::Communication; }
    if ["spotify.exe","vlc.exe","wmplayer.exe"].iter().any(|v| x == *v) { return Category::Media; }
    if ["chrome.exe","msedge.exe","firefox.exe","opera.exe","brave.exe"].iter().any(|v| x == *v) { return Category::Neutral; }
    Category::Neutral
}

#[cfg(windows)]
fn current_identity() -> Option<ForegroundInfo> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_NAME_WIN32};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() { return None; }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 { return None; }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buffer.as_mut_ptr()), &mut size).is_ok();
        let _ = CloseHandle(handle);
        if !ok { return None; }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        let exe = std::path::Path::new(&path).file_name()?.to_string_lossy().to_string();
        let app_name = std::path::Path::new(&exe).file_stem().unwrap_or_default().to_string_lossy().to_string();
        Some(ForegroundInfo { exe, app_name, pid })
    }
}
#[cfg(not(windows))]
fn current_identity() -> Option<ForegroundInfo> { None }

#[cfg(windows)]
fn start_windows_hook() {
    use windows::Win32::UI::Accessibility::SetWinEventHook;
    use windows::Win32::UI::WindowsAndMessaging::{EVENT_SYSTEM_FOREGROUND, GetMessageW, MSG, WINEVENT_OUTOFCONTEXT};
    unsafe {
        let _hook = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(win_event_proc), 0, 0, WINEVENT_OUTOFCONTEXT);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    }
}
#[cfg(windows)]
unsafe extern "system" fn win_event_proc(_: windows::Win32::UI::Accessibility::HWINEVENTHOOK, event: u32, _: windows::Win32::Foundation::HWND, _: i32, _: i32, _: u32, _: u32) {
    use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_FOREGROUND;
    if event != EVENT_SYSTEM_FOREGROUND { return; }
    if let Some(shared) = GLOBAL_TRACKER.get() { if let Ok(mut tracker) = shared.lock() { tracker.refresh_foreground(); tracker.enforce_active(); } }
}

#[tauri::command]
fn get_tracker_data(state: State<AppState>) -> TrackerData { state.0.lock().unwrap().current_data() }

#[tauri::command]
fn save_settings(app: AppHandle, state: State<AppState>, mut settings: AppSettings) -> TrackerData {
    let arc = state.0.clone();
    settings.displayName = settings.displayName.trim().chars().take(28).collect();
    if settings.displayName.is_empty() { settings.displayName = "Digital Balance".into(); }
    { let mut tracker = arc.lock().unwrap(); tracker.data.settings = settings.clone(); tracker.persist(); }
    let manager = app.autolaunch();
    if settings.autostartEnabled { let _ = manager.enable(); } else { let _ = manager.disable(); }
    let data = { let mut tracker = arc.lock().unwrap(); tracker.current_data() };
    data
}

#[tauri::command]
fn save_category(state: State<AppState>, exe: String, category: String) -> TrackerData {
    let mut tracker = state.0.lock().unwrap();
    let parsed = match category.as_str() { "Productive" => Category::Productive, "Distracting" => Category::Distracting, "Communication" => Category::Communication, "Media" => Category::Media, _ => Category::Neutral };
    tracker.data.settings.categoryOverrides.insert(exe.clone(), parsed.clone());
    if let Some(active) = tracker.active.as_mut() { if active.exe == exe { active.category = parsed.clone(); tracker.data.current_category = Some(parsed); } }
    tracker.persist(); tracker.current_data()
}

#[tauri::command]
fn set_tracking_paused(state: State<AppState>, paused: bool) -> TrackerData {
    let mut tracker = state.0.lock().unwrap();
    if paused { tracker.finish_active(); tracker.data.tracking_paused = true; tracker.data.current_app=None; tracker.data.current_exe=None; tracker.data.current_category=None; tracker.data.current_session_secs=0; tracker.active=None; }
    else { tracker.data.tracking_paused = false; tracker.refresh_foreground(); }
    tracker.persist(); tracker.current_data()
}

#[tauri::command]
fn complete_focus_session(_app: AppHandle, state: State<AppState>, minutes: u32, mode: String) -> Result<serde_json::Value, String> {
    let mut tracker = state.0.lock().map_err(|_| "tracker lock failed")?;
    let safe_minutes = minutes.clamp(5, 180);
    let recent_streak = tracker.data.focus_sessions.iter().rev().take(7).count() as u32;
    let points = safe_minutes;
    let coins = safe_minutes.saturating_mul(2).saturating_add(recent_streak.saturating_mul(5));
    let mode_enum = match mode.as_str() { "Deep Focus" => AppMode::DeepFocus, "Casual" => AppMode::Casual, _ => AppMode::Smart };
    tracker.data.focus_sessions.push(FocusSession { minutes: safe_minutes, points, coins, completed_at: Utc::now().to_rfc3339(), mode: mode_enum });
    tracker.data.settings.coins = tracker.data.settings.coins.saturating_add(coins);
    if tracker.data.focus_sessions.len() > 1000 { let drain = tracker.data.focus_sessions.len() - 1000; tracker.data.focus_sessions.drain(0..drain); }
    tracker.persist();
    let data = tracker.current_data();
    Ok(serde_json::json!({"points":points,"coins":coins,"data":data}))
}

#[tauri::command]
fn set_focus_protection(_app: AppHandle, state: State<AppState>, active: bool, blockedApps: Vec<String>, mode: String) -> TrackerData {
    let mut tracker = state.0.lock().unwrap();
    tracker.focus_active = active;
    tracker.focus_mode = if active { Some(match mode.as_str() { "Deep Focus" => AppMode::DeepFocus, "Casual" => AppMode::Casual, _ => AppMode::Smart }) } else { None };
    tracker.focus_started = if active { Some(Utc::now()) } else { None };
    tracker.focus_apps = blockedApps.into_iter().map(|x| x.to_ascii_lowercase()).filter(|x| safe_exe(x)).collect();
    tracker.casual_focus_started.clear();
    tracker.data.settings.deepFocusBlockedApps = tracker.focus_apps.iter().cloned().collect();
    tracker.persist();
    if active { tracker.refresh_foreground(); tracker.enforce_active(); } else { hide_limit_overlay(); }
    tracker.current_data()
}

#[tauri::command]
fn force_close_policy_target(state: State<AppState>, pid: u32, exe: String) -> Result<(), String> {
    let mut tracker = state.0.lock().map_err(|_| "tracker lock failed")?;
    if !safe_exe(&exe) { return Err("That process is not an allowed protection target.".into()); }
    tracker.close_pid(pid, &exe, "Closed by Digital Balance warning");
    tracker.warning_active = None;
    hide_limit_overlay();
    Ok(())
}

#[tauri::command]
fn close_policy_overlay() -> Result<(), String> { hide_limit_overlay(); Ok(()) }

#[tauri::command]
fn save_app_limit(state: State<AppState>, exe: String, limit: AppLimit) -> Result<TrackerData, String> {
    if !safe_exe(&exe) { return Err("That executable is not allowed as a protection target.".into()); }
    if limit.scope == LimitScope::UntilDate {
        let Some(expires) = limit.expiresAt.as_deref() else { return Err("Choose an expiry date for this protection.".into()); };
        let expires = DateTime::parse_from_rfc3339(expires).map_err(|_| "Invalid expiry date.")?;
        if expires.with_timezone(&Utc) <= Utc::now() { return Err("The expiry date must be in the future.".into()); }
    }
    let mut tracker = state.0.lock().unwrap();
    let normalized = AppLimit {
        dailyMinutes: limit.dailyMinutes.max(1),
        action: limit.action,
        enabled: limit.enabled,
        scope: limit.scope,
        startedAt: Some(Utc::now().to_rfc3339()),
        expiresAt: limit.expiresAt,
    };
    tracker.data.settings.appLimits.insert(exe.clone(), normalized);
    tracker.warn_pending.remove(&exe.to_ascii_lowercase());
    tracker.warn_reopen_counts.remove(&exe.to_ascii_lowercase());
    tracker.warning_active = None;
    tracker.persist();
    tracker.enforce_active();
    Ok(tracker.current_data())
}

#[tauri::command]
fn remove_app_limit(state: State<AppState>, exe: String) -> TrackerData { let mut tracker=state.0.lock().unwrap(); tracker.data.settings.appLimits.remove(&exe); tracker.persist(); tracker.current_data() }

#[tauri::command]
fn purchase_reward(state: State<AppState>, rewardId: String) -> Result<TrackerData, String> {
    let price = reward_price(&rewardId).ok_or_else(|| "Unknown reward.".to_string())?;
    let mut tracker = state.0.lock().unwrap();
    if tracker.data.settings.ownedRewards.iter().any(|x| x == &rewardId) { return Ok(tracker.current_data()); }
    if tracker.data.settings.coins < price { return Err("You need more Balance Coins.".into()); }
    tracker.data.settings.coins -= price;
    tracker.data.settings.ownedRewards.push(rewardId.clone());
    if rewardId.starts_with("theme:") { tracker.data.settings.purchasedThemes.push(rewardId.clone()); }
    tracker.persist(); Ok(tracker.current_data())
}

#[tauri::command]
fn equip_reward(state: State<AppState>, rewardId: String) -> Result<TrackerData, String> {
    if reward_price(&rewardId).is_none() { return Err("Unknown reward.".into()); }
    let mut tracker = state.0.lock().unwrap();
    if !tracker.data.settings.ownedRewards.contains(&rewardId) { return Err("Unlock this reward first.".into()); }
    if rewardId.starts_with("theme:") { tracker.data.settings.theme = rewardId.clone(); }
    else if rewardId.starts_with("frame:") { tracker.data.settings.equippedFrame = rewardId.clone(); }
    else if rewardId.starts_with("effect:") { tracker.data.settings.equippedEffect = rewardId.clone(); }
    tracker.persist(); Ok(tracker.current_data())
}

#[tauri::command]
fn delete_local_data(state: State<AppState>) -> TrackerData { let mut tracker=state.0.lock().unwrap(); tracker.data.sessions.clear(); tracker.data.focus_sessions.clear(); tracker.data.settings.appLimits.clear(); tracker.data.current_app=None; tracker.data.current_exe=None; tracker.data.current_category=None; tracker.data.current_session_secs=0; tracker.persist(); tracker.current_data() }
#[tauri::command]
fn export_local_data(state: State<AppState>) -> String { serde_json::to_string_pretty(&state.0.lock().unwrap().data).unwrap_or_else(|_| "{}".into()) }

#[tauri::command]
fn hide_to_tray(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
.plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .setup(|app| {
            #[cfg(windows)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register_all();
            }

            // Build the system tray natively in Rust. This avoids relying on the
            // frontend webview to create the tray resource when entering background mode.
            let open_item = MenuItemBuilder::with_id("open", "Open Digital Balance").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit Digital Balance").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&open_item)
                .separator()
                .item(&quit_item)
                .build()?;
            let tray_icon = app.default_window_icon().cloned()
                .expect("Digital Balance default icon is required for the system tray");
            TrayIconBuilder::with_id("digital-balance-tray")
                .icon(tray_icon)
                .tooltip("Digital Balance — running in the background")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            let path = app.path().app_data_dir()?.join("digital-balance.json");
            let tracker = Tracker::load(path);
            let shared = Arc::new(Mutex::new(tracker));
            app.manage(AppState(shared.clone()));
            let _ = GLOBAL_TRACKER.set(shared.clone());
            let _ = GLOBAL_APP.set(app.handle().clone());
            #[cfg(windows)]
            { thread::spawn(start_windows_hook); }
            thread::spawn(|| loop {
                thread::sleep(Duration::from_secs(5));
                if let Some(shared)=GLOBAL_TRACKER.get() {
                    if let Ok(mut tracker)=shared.lock() {
                        tracker.refresh_foreground();
                        tracker.enforce_active();
                    }
                }
            });
            Ok(())
        });
    builder.invoke_handler(tauri::generate_handler![
        get_tracker_data, save_settings, save_category, set_tracking_paused,
        complete_focus_session, set_focus_protection, force_close_policy_target,
        close_policy_overlay, save_app_limit, remove_app_limit, purchase_reward,
        equip_reward, delete_local_data, export_local_data, hide_to_tray, quit_app
    ])
        .run(tauri::generate_context!())
        .expect("error while running Digital Balance");
}
