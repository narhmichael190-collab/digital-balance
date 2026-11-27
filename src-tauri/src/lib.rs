#![allow(non_snake_case)]

use chrono::{DateTime, Local, Utc};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt as AutoStartManagerExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AppMode {
    Casual,
    Smart,
    #[serde(rename = "Deep Focus")]
    DeepFocus,
}
impl Default for AppMode {
    fn default() -> Self {
        Self::Smart
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Category {
    Productive,
    Distracting,
    Communication,
    Media,
    Neutral,
}
impl Default for Category {
    fn default() -> Self {
        Self::Neutral
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LimitAction {
    Warn,
    Close,
    Block,
}
impl Default for LimitAction {
    fn default() -> Self {
        Self::Block
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitScope {
    Daily,
    UntilDate,
}
impl Default for LimitScope {
    fn default() -> Self {
        Self::Daily
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLimit {
    pub dailyMinutes: u32,
    #[serde(default)]
    pub action: LimitAction,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub scope: LimitScope,
    #[serde(default)]
    pub startedAt: Option<String>,
    #[serde(default)]
    pub expiresAt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub dailyDistractionLimitMinutes: u32,
    pub dailyFocusTargetMinutes: u32,
    pub idleThresholdSeconds: u64,
    pub mode: AppMode,
    pub notificationsEnabled: bool,
    pub autostartEnabled: bool,
    #[serde(default = "default_true")]
    pub closeToTray: bool,
    #[serde(default = "default_true")]
    pub motivationalNudges: bool,
    #[serde(default = "default_display_name")]
    pub displayName: String,
    pub categoryOverrides: HashMap<String, Category>,
    #[serde(default)]
    pub appLimits: HashMap<String, AppLimit>,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub purchasedThemes: Vec<String>,
    #[serde(default)]
    pub coins: u32,
    #[serde(default)]
    pub ownedRewards: Vec<String>,
    #[serde(default = "default_frame")]
    pub equippedFrame: String,
    #[serde(default = "default_effect")]
    pub equippedEffect: String,
    #[serde(default)]
    pub deepFocusBlockedApps: Vec<String>,
    #[serde(default = "default_true")]
    pub deepFocusAutoBlock: bool,
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
            displayName: "Liba Bilancia".into(),
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
fn default_display_name() -> String { "Liba Bilancia".into() }
fn default_theme() -> String { "theme:obsidian".into() }
fn default_frame() -> String { "frame:classic".into() }
fn default_effect() -> String { "effect:clean".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub app_name: String,
    pub exe: String,
    pub category: Category,
    pub started_at: String,
    pub ended_at: String,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSession {
    pub minutes: u32,
    pub points: u32,
    pub coins: u32,
    pub completed_at: String,
    pub mode: AppMode,
}

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
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            focus_sessions: Vec::new(),
            current_app: None,
            current_exe: None,
            current_category: None,
            current_session_secs: 0,
            tracking_paused: false,
            settings: AppSettings::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct ActiveSession {
    exe: String,
    app_name: String,
    category: Category,
    pid: u32,
    started: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellbeingRecord {
    pub id: String,
    pub completed_at: String,
    pub duration_secs: u64,
    pub coins: u32,
}

#[derive(Debug, Clone)]
struct ActiveWellbeing {
    id: String,
    started: Instant,
    completed_steps: u8,
}

#[derive(Debug)]
struct Tracker {
    data: TrackerData,
    path: PathBuf,
    active: Option<ActiveSession>,
    focus_active: bool,
    focus_mode: Option<AppMode>,
    focus_started: Option<DateTime<Utc>>,
    focus_apps: HashSet<String>,
    casual_focus_started: HashMap<String, DateTime<Utc>>,
    warn_observed: HashSet<String>,
    warn_reopen_counts: HashMap<String, u8>,
    warning_active: Option<(String, u32)>,
    close_triggered: HashSet<String>,
    wellbeing_records: Vec<WellbeingRecord>,
    wellbeing_active: Option<ActiveWellbeing>,
    closed_date: String,
}

#[derive(Clone)]
pub struct AppState(Arc<Mutex<Tracker>>);

static GLOBAL_TRACKER: OnceCell<Arc<Mutex<Tracker>>> = OnceCell::new();
static GLOBAL_APP: OnceCell<AppHandle> = OnceCell::new();

fn local_day() -> String {
    Local::now().date_naive().to_string()
}

fn safe_exe(exe: &str) -> bool {
    let lower = exe.to_ascii_lowercase();
    if exe.is_empty() || exe.len() > 160 {
        return false;
    }
    if !lower.ends_with(".exe") {
        return false;
    }
    if !exe
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return false;
    }

    !matches!(
        lower.as_str(),
        "system.exe"
            | "system idle process.exe"
            | "smss.exe"
            | "csrss.exe"
            | "wininit.exe"
            | "winlogon.exe"
            | "services.exe"
            | "lsass.exe"
            | "svchost.exe"
            | "dwm.exe"
            | "explorer.exe"
            | "digital-balance.exe"
            | "liba-bilancia.exe"
            | "lockapp.exe"
            | "shellhost.exe"
            | "shellexperiencehost.exe"
            | "startmenuexperiencehost.exe"
            | "searchhost.exe"
            | "searchapp.exe"
            | "applicationframehost.exe"
            | "textinputhost.exe"
            | "runtimebroker.exe"
            | "dllhost.exe"
            | "conhost.exe"
            | "fontdrvhost.exe"
            | "sihost.exe"
            | "ctfmon.exe"
            | "taskhostw.exe"
            | "wmiprvse.exe"
            | "searchindexer.exe"
            | "spoolsv.exe"
            | "smartscreen.exe"
            | "msedgewebview2.exe"
            | "backgroundtaskhost.exe"
            | "widgetservice.exe"
            | "widgets.exe"
            | "securityhealthservice.exe"
            | "securityhealthsystray.exe"
            | "audiodg.exe"
    )
}

fn normalize_exe_target(exe: &str) -> String {
    std::path::Path::new(exe)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}

fn wellbeing_duration_secs(id: &str) -> Option<u64> {
    match id {
        "box-breathing" => Some(180),
        "grounding-5" => Some(240),
        "desk-stretch" => Some(360),
        "morning-flow" => Some(480),
        "march-break" => Some(300),
        "balance-break" => Some(240),
        "walk-reset" => Some(600),
        "desk-reset" => Some(180),
        _ => None,
    }
}

fn wellbeing_coins(duration_secs: u64) -> u32 {
    (duration_secs / 60).saturating_mul(2).clamp(1, 20) as u32
}

fn wellbeing_day_records(records: &[WellbeingRecord]) -> Vec<WellbeingRecord> {
    let today = local_day();
    records
        .iter()
        .filter(|r| r.completed_at.get(..10).unwrap_or("") == today)
        .cloned()
        .collect()
}

fn free_theme_ids() -> Vec<String> {
    [
        "theme:obsidian",
        "theme:aurora",
        "theme:ember",
        "theme:forest",
        "theme:ocean",
        "theme:violet",
        "theme:rose",
        "theme:sunset",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
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
        "theme:obsidian"
        | "theme:aurora"
        | "theme:ember"
        | "theme:forest"
        | "theme:ocean"
        | "theme:violet"
        | "theme:rose"
        | "theme:sunset" => 0,
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
        let mut data = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<TrackerData>(&raw).ok())
            .unwrap_or_default();

        if data.settings.purchasedThemes.is_empty() {
            data.settings.purchasedThemes = free_theme_ids();
        }

        let mut owned = if data.settings.ownedRewards.is_empty() {
            data.settings.purchasedThemes.clone()
        } else {
            data.settings.ownedRewards.clone()
        };

        for id in free_theme_ids() {
            if !owned.contains(&id) {
                owned.push(id);
            }
        }

        data.settings.ownedRewards = owned;

        if data.settings.displayName.trim().is_empty()
            || data.settings.displayName == "Digital Balance"
        {
            data.settings.displayName = "Liba Bilancia".into();
        }

        if data.settings.theme.is_empty() {
            data.settings.theme = "theme:obsidian".into();
        }

        Self {
            data,
            path,
            active: None,
            focus_active: false,
            focus_mode: None,
            focus_started: None,
            focus_apps: HashSet::new(),
            casual_focus_started: HashMap::new(),
            warn_observed: HashSet::new(),
            warn_reopen_counts: HashMap::new(),
            warning_active: None,
            close_triggered: HashSet::new(),
            wellbeing_records: Vec::new(),
            wellbeing_active: None,
            closed_date: local_day(),
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(raw) = serde_json::to_string_pretty(&self.data) {
            let _ = fs::write(&self.path, raw);
        }
    }

    fn normalize_day(&mut self) {
        let now = local_day();

        if now != self.closed_date {
            self.closed_date = now;
            self.warn_observed.clear();
            self.warn_reopen_counts.clear();
            self.close_triggered.clear();
            self.warning_active = None;
            hide_limit_overlay();
        }
    }

    fn category_for(&self, exe: &str) -> Category {
        self.data
            .settings
            .categoryOverrides
            .get(exe)
            .cloned()
            .unwrap_or_else(|| auto_category(exe))
    }

    fn finish_active(&mut self) {
        if let Some(active) = self.active.take() {
            let seconds = (Utc::now() - active.started).num_seconds().max(0) as u64;
            self.data.current_session_secs = 0;

            if seconds >= 1 {
                self.data.sessions.push(SessionRecord {
                    app_name: active.app_name,
                    exe: active.exe,
                    category: active.category,
                    started_at: active.started.to_rfc3339(),
                    ended_at: Utc::now().to_rfc3339(),
                    duration_secs: seconds,
                });

                if self.data.sessions.len() > 4000 {
                    let drain = self.data.sessions.len() - 4000;
                    self.data.sessions.drain(0..drain);
                }

                self.persist();
            }
        }
    }

    fn usage_today(&self, exe: &str) -> u64 {
        let day = Local::now().date_naive();
        let mut total = 0u64;

        for session in self
            .data
            .sessions
            .iter()
            .filter(|s| s.exe.eq_ignore_ascii_case(exe))
        {
            if let (Ok(start), Ok(end)) = (
                DateTime::parse_from_rfc3339(&session.started_at),
                DateTime::parse_from_rfc3339(&session.ended_at),
            ) {
                if start.with_timezone(&Local).date_naive() == day {
                    total = total.saturating_add(
                        (end - start).num_seconds().max(0) as u64
                    );
                }
            }
        }

        if let Some(active) = &self.active {
            if active.exe.eq_ignore_ascii_case(exe) {
                total = total.saturating_add(
                    (Utc::now() - active.started).num_seconds().max(0) as u64
                );
            }
        }

        total
    }

    fn usage_since(&self, exe: &str, since: DateTime<Utc>) -> u64 {
        let now = Utc::now();
        let mut total = 0u64;

        for session in self
            .data
            .sessions
            .iter()
            .filter(|s| s.exe.eq_ignore_ascii_case(exe))
        {
            if let (Ok(start), Ok(end)) = (
                DateTime::parse_from_rfc3339(&session.started_at),
                DateTime::parse_from_rfc3339(&session.ended_at),
            ) {
                let start = start.with_timezone(&Utc);
                let end = end.with_timezone(&Utc);
                let effective_start = if start < since { since } else { start };
                let effective_end = if end > now { now } else { end };

                if effective_end > effective_start {
                    total = total.saturating_add(
                        (effective_end - effective_start).num_seconds().max(0) as u64
                    );
                }
            }
        }

        if let Some(active) = &self.active {
            if active.exe.eq_ignore_ascii_case(exe) {
                let effective_start = if active.started < since {
                    since
                } else {
                    active.started
                };

                if now > effective_start {
                    total = total.saturating_add(
                        (now - effective_start).num_seconds().max(0) as u64
                    );
                }
            }
        }

        total
    }

    fn usage_for_rule(&self, exe: &str, rule: &AppLimit) -> u64 {
        match rule.scope {
            LimitScope::Daily => self.usage_today(exe),
            LimitScope::UntilDate => {
                let since = rule
                    .startedAt
                    .as_deref()
                    .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                self.usage_since(exe, since)
            }
        }
    }

    fn refresh_foreground(&mut self) {
        if self.data.tracking_paused {
            return;
        }

        if let Some(info) = current_identity() {
            if self.active.as_ref().map(|x| x.pid) != Some(info.pid) {
                self.finish_active();

                let category = self.category_for(&info.exe);

                self.active = Some(ActiveSession {
                    exe: info.exe.clone(),
                    app_name: info.app_name.clone(),
                    category: category.clone(),
                    pid: info.pid,
                    started: Utc::now(),
                });

                self.data.current_app = Some(info.app_name);
                self.data.current_exe = Some(info.exe.clone());
                self.data.current_category = Some(category);
                self.data.current_session_secs = 0;
                self.persist();
            }
        }
    }

    // Return a cheap UI snapshot only. Foreground tracking and process enforcement
    // run on the dedicated background worker, never as a side effect of a UI read.
    fn current_data(&self) -> TrackerData {
        let mut out = self.data.clone();

        if let Some(active) = &self.active {
            out.current_session_secs =
                (Utc::now() - active.started).num_seconds().max(0) as u64;
        } else {
            out.current_session_secs = 0;
        }

        out.tracking_paused = self.data.tracking_paused;
        out
    }

    fn enforce_active(&mut self) {
        self.normalize_day();

        if self.data.tracking_paused {
            return;
        }

        let processes = enumerate_processes();
        let visible_pids = enumerate_visible_process_ids();

        let visible_exes: HashSet<String> = processes
            .iter()
            .filter(|p| visible_pids.contains(&p.pid))
            .map(|p| p.exe.to_ascii_lowercase())
            .collect();

        self.warn_observed
            .retain(|key| visible_exes.contains(key));

        if let Some((warning_exe, warning_pid)) = self.warning_active.clone() {
            if !visible_exes.contains(&warning_exe) || !process_exists(warning_pid) {
                self.warning_active = None;
                hide_limit_overlay();
            }
        }

        if self.focus_active {
            let mode = self.focus_mode.clone().unwrap_or(AppMode::Smart);

            for blocked in self.focus_apps.clone() {
                if !processes
                    .iter()
                    .any(|p| p.exe.eq_ignore_ascii_case(&blocked))
                {
                    continue;
                }

                match mode {
                    AppMode::DeepFocus | AppMode::Smart => {
                        self.close_matching_exe(&blocked, "Focus protection");
                    }
                    AppMode::Casual => {
                        let started = self
                            .casual_focus_started
                            .entry(blocked.clone())
                            .or_insert_with(Utc::now);

                        let grace =
                            (Utc::now() - *started).num_seconds().max(0) as u64;

                        if grace >= 10 * 60 {
                            if let Some(handle) = GLOBAL_APP.get() {
                                let _ = handle.emit(
                                    "focus-grace-expired",
                                    serde_json::json!({
                                        "app": friendly_process_name(&blocked),
                                        "limitMinutes": 10
                                    }),
                                );
                            }

                            self.close_matching_exe(
                                &blocked,
                                "Casual Focus grace period ended",
                            );
                        }
                    }
                }
            }
        }

        let rules: Vec<(String, AppLimit)> = self
            .data
            .settings
            .appLimits
            .iter()
            .map(|(exe, rule)| (exe.clone(), rule.clone()))
            .collect();

        for (configured_exe, rule) in rules {
            if !rule.enabled {
                continue;
            }

            if rule.scope == LimitScope::UntilDate {
                if let Some(expires) = rule
                    .expiresAt
                    .as_deref()
                    .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                {
                    if Utc::now() >= expires.with_timezone(&Utc) {
                        continue;
                    }
                }
            }

            let key = normalize_exe_target(&configured_exe);

            let matches: Vec<ProcessInfo> = processes
                .iter()
                .filter(|p| p.exe.eq_ignore_ascii_case(&configured_exe))
                .cloned()
                .collect();

            if matches.is_empty() {
                continue;
            }

            let visible_matches: Vec<ProcessInfo> = matches
                .iter()
                .filter(|p| visible_pids.contains(&p.pid))
                .cloned()
                .collect();

            let used_seconds = self.usage_for_rule(&configured_exe, &rule);
            let limit_seconds = u64::from(rule.dailyMinutes).saturating_mul(60);

            if used_seconds < limit_seconds {
                continue;
            }

            match rule.action {
                LimitAction::Warn => {
                    if !self.warn_observed.contains(&key) && !visible_matches.is_empty() {
                        let count = {
                            let entry =
                                self.warn_reopen_counts.entry(key.clone()).or_insert(0);
                            *entry = entry.saturating_add(1);
                            *entry
                        };

                        self.warn_observed.insert(key.clone());

                        if count > 3 {
                            self.close_matching_exe(
                                &configured_exe,
                                "Three warnings used - protection is now blocking this app",
                            );
                            self.warning_active = None;
                            hide_limit_overlay();
                        } else {
                            let pid = visible_matches[0].pid;
                            self.warning_active = Some((key.clone(), pid));

                            let app_name = friendly_process_name(&configured_exe);

                            show_limit_overlay(
                                &configured_exe,
                                &app_name,
                                pid,
                                count,
                            );

                            self.emit_limit(
                                configured_exe.clone(),
                                app_name,
                                format!(
                                    "Warning {} of 3 - close this app to continue",
                                    count
                                ),
                                used_seconds,
                                limit_seconds,
                            );
                        }
                    }
                }

                LimitAction::Close => {
                    if !self.close_triggered.contains(&key) {
                        self.close_triggered.insert(key.clone());
                        self.close_matching_exe(
                            &configured_exe,
                            "Limit reached - app closed",
                        );
                    }
                }

                LimitAction::Block => {
                    self.close_matching_exe(
                        &configured_exe,
                        "Limit reached - app blocked until the rule resets",
                    );
                }
            }
        }
    }

    fn emit_limit(
        &self,
        exe: String,
        app: String,
        reason: String,
        used: u64,
        limit: u64,
    ) {
        if let Some(handle) = GLOBAL_APP.get() {
            let _ = handle.emit(
                "app-limit-hit",
                serde_json::json!({
                    "exe": exe,
                    "app": app,
                    "reason": reason,
                    "usedMinutes": used / 60,
                    "limitMinutes": limit / 60
                }),
            );
        }
    }

    fn close_matching_exe(&mut self, exe: &str, reason: &str) {
        let exe = normalize_exe_target(exe);

        if !safe_exe(&exe) {
            return;
        }

        let pids: Vec<u32> = enumerate_processes()
            .into_iter()
            .filter(|p| p.exe.eq_ignore_ascii_case(&exe))
            .map(|p| p.pid)
            .collect();

        for pid in pids {
            terminate_process_tree(pid, &exe);
        }

        let should_finish = self
            .active
            .as_ref()
            .map(|active| {
                active.exe.eq_ignore_ascii_case(&exe)
                    || !process_exists(active.pid)
            })
            .unwrap_or(false);

        if should_finish {
            self.finish_active();
            self.active = None;
            self.data.current_app = None;
            self.data.current_exe = None;
            self.data.current_category = None;
            self.data.current_session_secs = 0;
        }

        let (used, limit) = self
            .data
            .settings
            .appLimits
            .iter()
            .find(|(configured, _)| normalize_exe_target(configured) == exe)
            .map(|(_, rule)| {
                (
                    self.usage_for_rule(&exe, rule),
                    u64::from(rule.dailyMinutes).saturating_mul(60),
                )
            })
            .unwrap_or((0, 0));

        self.emit_limit(
            exe.clone(),
            friendly_process_name(&exe),
            reason.to_string(),
            used,
            limit,
        );
    }
}

fn encode_url(input: &str) -> String {
    input
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') {
                (b as char).to_string()
            } else {
                format!("%{:02X}", b)
            }
        })
        .collect()
}

fn show_limit_overlay(exe: &str, app_name: &str, pid: u32, reopen: u8) {
    let Some(app) = GLOBAL_APP.get() else {
        return;
    };

    let _ = app
        .get_webview_window("limit-warning")
        .map(|w| w.close());

    let url = format!(
        "overlay.html?type=limit&app={}&exe={}&pid={}&reopen={}",
        encode_url(app_name),
        encode_url(exe),
        pid,
        reopen
    );

    let _ = WebviewWindowBuilder::new(
        app,
        "limit-warning",
        WebviewUrl::App(url.into()),
    )
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(true)
    .fullscreen(true)
    .build();
}

fn hide_limit_overlay() {
    if let Some(app) = GLOBAL_APP.get() {
        let _ = app
            .get_webview_window("limit-warning")
            .map(|w| w.close());
    }
}

fn process_exists(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Ok(handle) => {
                    let _ = CloseHandle(handle);
                    true
                }
                Err(_) => false,
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = pid;
        false
    }
}

#[derive(Clone, Debug)]
struct ProcessInfo {
    pid: u32,
    exe: String,
}

#[cfg(windows)]
fn enumerate_processes() -> Vec<ProcessInfo> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot,
        Process32FirstW,
        Process32NextW,
        PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let snapshot =
        unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

    let Ok(snapshot) = snapshot else {
        return Vec::new();
    };

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    let mut result = Vec::new();

    unsafe {
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());

                let exe =
                    String::from_utf16_lossy(&entry.szExeFile[..end]);

                if entry.th32ProcessID != 0 && safe_exe(&exe) {
                    result.push(ProcessInfo {
                        pid: entry.th32ProcessID,
                        exe,
                    });
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    result
}

#[cfg(not(windows))]
fn enumerate_processes() -> Vec<ProcessInfo> {
    Vec::new()
}

#[cfg(windows)]
fn terminate_process_tree(pid: u32, exe: &str) {
    if !safe_exe(exe) || pid == 0 {
        return;
    }

    let killed = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(0x08000000)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if killed {
        return;
    }

    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess,
        TerminateProcess,
        PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE,
    };

    unsafe {
        if let Ok(handle) = OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        ) {
            let _ = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);
        }
    }
}

#[cfg(not(windows))]
fn terminate_process_tree(pid: u32, _exe: &str) {
    let _ = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status();
}

#[derive(Clone)]
struct ForegroundInfo {
    exe: String,
    app_name: String,
    pid: u32,
}

fn auto_category(exe: &str) -> Category {
    let x = exe.to_ascii_lowercase();

    if [
        "code.exe",
        "devenv.exe",
        "idea64.exe",
        "pycharm64.exe",
        "notepad.exe",
        "excel.exe",
        "winword.exe",
        "powershell.exe",
        "cmd.exe",
    ]
    .iter()
    .any(|v| x == *v)
    {
        return Category::Productive;
    }

    if ["discord.exe", "whatsapp.exe", "telegram.exe", "slack.exe"]
        .iter()
        .any(|v| x == *v)
    {
        return Category::Communication;
    }

    if ["spotify.exe", "vlc.exe", "wmplayer.exe"]
        .iter()
        .any(|v| x == *v)
    {
        return Category::Media;
    }

    if ["chrome.exe", "msedge.exe", "firefox.exe", "opera.exe", "brave.exe"]
        .iter()
        .any(|v| x == *v)
    {
        return Category::Neutral;
    }

    Category::Neutral
}

#[cfg(windows)]
fn current_identity() -> Option<ForegroundInfo> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess,
        QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow,
        GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();

        if hwnd.is_invalid() {
            return None;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        if pid == 0 {
            return None;
        }

        let handle =
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;

        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .is_ok();

        let _ = CloseHandle(handle);

        if !ok {
            return None;
        }

        let path =
            String::from_utf16_lossy(&buffer[..size as usize]);

        let exe = std::path::Path::new(&path)
            .file_name()?
            .to_string_lossy()
            .to_string();

        if !safe_exe(&exe) {
            return None;
        }

        let app_name = std::path::Path::new(&exe)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Some(ForegroundInfo {
            exe,
            app_name,
            pid,
        })
    }
}

#[cfg(not(windows))]
fn current_identity() -> Option<ForegroundInfo> {
    None
}

#[cfg(windows)]
fn start_windows_hook() {
    use windows::Win32::UI::Accessibility::SetWinEventHook;
    use windows::Win32::UI::WindowsAndMessaging::{
        EVENT_SYSTEM_FOREGROUND,
        GetMessageW,
        MSG,
        WINEVENT_OUTOFCONTEXT,
    };

    unsafe {
        let _hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        let mut msg = MSG::default();

        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
    }
}

#[cfg(windows)]
unsafe extern "system" fn win_event_proc(
    _: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    event: u32,
    _: windows::Win32::Foundation::HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    use windows::Win32::UI::WindowsAndMessaging::EVENT_SYSTEM_FOREGROUND;

    if event != EVENT_SYSTEM_FOREGROUND {
        return;
    }

    if let Some(shared) = GLOBAL_TRACKER.get() {
        if let Ok(mut tracker) = shared.try_lock() {
            // The 1-second worker handles enforcement. Keep this event hook cheap
            // and non-blocking so Windows foreground events never wait on process work.
            tracker.refresh_foreground();
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunningApp {
    name: String,
    exe: String,
    installed: bool,
}

fn friendly_process_name(exe: &str) -> String {
    match exe.to_ascii_lowercase().as_str() {
        "chrome.exe" => "Google Chrome".into(),
        "msedge.exe" => "Microsoft Edge".into(),
        "firefox.exe" => "Mozilla Firefox".into(),
        "code.exe" => "Visual Studio Code".into(),
        "powerpnt.exe" => "PowerPoint".into(),
        "winword.exe" => "Word".into(),
        "excel.exe" => "Excel".into(),
        "vlc.exe" => "VLC media player".into(),
        "snippingtool.exe" => "Snipping Tool".into(),
        "windowsterminal.exe" => "Windows Terminal".into(),
        "telegram.exe" => "Telegram".into(),
        "discord.exe" => "Discord".into(),
        "spotify.exe" => "Spotify".into(),
        "steam.exe" => "Steam".into(),
        _ => std::path::Path::new(exe)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    }
}

#[cfg(windows)]
fn enumerate_visible_process_ids() -> HashSet<u32> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows,
        GetWindowTextLengthW,
        GetWindowThreadProcessId,
        IsWindowVisible,
    };

    unsafe extern "system" fn collect(
        hwnd: HWND,
        context: LPARAM,
    ) -> BOOL {
        if IsWindowVisible(hwnd).as_bool() && GetWindowTextLengthW(hwnd) > 0 {
            let mut pid = 0u32;

            GetWindowThreadProcessId(hwnd, Some(&mut pid));

            if pid != 0 {
                let ids =
                    &mut *(context.0 as *mut HashSet<u32>);
                ids.insert(pid);
            }
        }

        BOOL(1)
    }

    let mut ids = HashSet::new();

    unsafe {
        let _ = EnumWindows(
            Some(collect),
            LPARAM(&mut ids as *mut HashSet<u32> as isize),
        );
    }

    ids
}

#[cfg(not(windows))]
fn enumerate_visible_process_ids() -> HashSet<u32> {
    HashSet::new()
}

#[cfg(windows)]
fn running_apps_for_windows() -> Vec<RunningApp> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess,
        QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let pids = enumerate_visible_process_ids();
    let mut apps: HashMap<String, RunningApp> = HashMap::new();

    for pid in pids {
        unsafe {
            let Ok(handle) =
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            else {
                continue;
            };

            let mut buffer = [0u16; 1024];
            let mut size = buffer.len() as u32;

            let result = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buffer.as_mut_ptr()),
                &mut size,
            );

            let _ = CloseHandle(handle);

            if result.is_err() {
                continue;
            }

            let path =
                String::from_utf16_lossy(&buffer[..size as usize]);

            let exe = std::path::Path::new(&path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if !safe_exe(&exe) {
                continue;
            }

            let key = exe.to_ascii_lowercase();

            apps.entry(key).or_insert_with(|| RunningApp {
                name: friendly_process_name(&exe),
                exe,
                installed: false,
            });
        }
    }

    let mut result: Vec<RunningApp> = apps.into_values().collect();

    result.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });

    result
}

#[cfg(not(windows))]
fn running_apps_for_windows() -> Vec<RunningApp> {
    Vec::new()
}

#[cfg(windows)]
fn installed_apps_for_windows() -> Vec<RunningApp> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let script = r#"
$roots = @(
  "$env:APPDATA\Microsoft\Windows\Start Menu\Programs",
  "$env:USERPROFILE\Desktop",
  "$env:ProgramData\Microsoft\Windows\Start Menu\Programs",
  "$env:PUBLIC\Desktop"
) | Where-Object { Test-Path $_ } | Select-Object -Unique

$ws = New-Object -ComObject WScript.Shell

$items = foreach ($root in $roots) {
  Get-ChildItem -Path $root -Filter *.lnk -File -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
    try {
      $shortcut = $ws.CreateShortcut($_.FullName)
      $target = $shortcut.TargetPath

      if (
        $target -and
        [IO.Path]::GetExtension($target) -ieq '.exe' -and
        ($target -notlike "$env:WINDIR\*")
      ) {
        $exe = [IO.Path]::GetFileName($target)

        if ($exe -notmatch '^(?i:uninstall|unins.*|setup|update|updater|bootstrapper).*\.exe$') {
          [PSCustomObject]@{
            Name = $_.BaseName
            Exe  = $exe
          }
        }
      }
    } catch {}
  }
}

@($items | Where-Object { $_.Exe }) |
  Sort-Object Exe -Unique |
  ConvertTo-Json -Compress
"#;

    let output = std::process::Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            script,
        ])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);

    let value: serde_json::Value =
        serde_json::from_str(text.trim())
            .unwrap_or(serde_json::Value::Array(Vec::new()));

    let array = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![value],
        _ => Vec::new(),
    };

    let mut apps = Vec::new();

    for item in array {
        let Some(exe) = item.get("Exe").and_then(|v| v.as_str()) else {
            continue;
        };

        let name = item
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(exe);

        if safe_exe(exe) {
            apps.push(RunningApp {
                name: if name.trim().is_empty() {
                    friendly_process_name(exe)
                } else {
                    name.to_string()
                },
                exe: exe.to_string(),
                installed: true,
            });
        }
    }

    apps
}

#[cfg(not(windows))]
fn installed_apps_for_windows() -> Vec<RunningApp> {
    Vec::new()
}

fn protection_apps_for_windows() -> Vec<RunningApp> {
    let mut merged: HashMap<String, RunningApp> = HashMap::new();

    for app in running_apps_for_windows()
        .into_iter()
        .chain(installed_apps_for_windows())
    {
        if !safe_exe(&app.exe) {
            continue;
        }

        let key = app.exe.to_ascii_lowercase();
        merged.entry(key).or_insert(app);
    }

    for app in [
        RunningApp {
            name: "Google Chrome".into(),
            exe: "chrome.exe".into(),
            installed: false,
        },
        RunningApp {
            name: "Microsoft Edge".into(),
            exe: "msedge.exe".into(),
            installed: false,
        },
        RunningApp {
            name: "Mozilla Firefox".into(),
            exe: "firefox.exe".into(),
            installed: false,
        },
        RunningApp {
            name: "Discord".into(),
            exe: "discord.exe".into(),
            installed: false,
        },
        RunningApp {
            name: "Spotify".into(),
            exe: "spotify.exe".into(),
            installed: false,
        },
        RunningApp {
            name: "Steam".into(),
            exe: "steam.exe".into(),
            installed: false,
        },
        RunningApp {
            name: "Telegram".into(),
            exe: "telegram.exe".into(),
            installed: false,
        },
        RunningApp {
            name: "VLC media player".into(),
            exe: "vlc.exe".into(),
            installed: false,
        },
    ] {
        merged.entry(app.exe.to_ascii_lowercase()).or_insert(app);
    }

    let mut result: Vec<RunningApp> = merged.into_values().collect();

    result.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });

    result
}

#[tauri::command]
async fn get_protection_apps() -> Vec<RunningApp> {
    tauri::async_runtime::spawn_blocking(protection_apps_for_windows)
        .await
        .unwrap_or_default()
}

#[tauri::command]
fn get_running_apps() -> Vec<RunningApp> {
    running_apps_for_windows()
}

#[tauri::command]
async fn get_tracker_data(state: State<'_, AppState>) -> Result<TrackerData, String> {
    let shared = state.0.clone();

    tauri::async_runtime::spawn_blocking(move || {
        shared
            .lock()
            .map(|tracker| tracker.current_data())
            .map_err(|_| "The tracker data lock is unavailable.".to_string())
    })
    .await
    .map_err(|_| "The tracker data worker failed.".to_string())?
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    mut settings: AppSettings,
) -> TrackerData {
    let arc = state.0.clone();

    settings.displayName = settings
        .displayName
        .trim()
        .chars()
        .take(28)
        .collect();

    if settings.displayName.is_empty() {
        settings.displayName = "Liba Bilancia".into();
    }

    {
        let mut tracker = arc.lock().unwrap();

        let coins = tracker.data.settings.coins;
        let owned_rewards = tracker.data.settings.ownedRewards.clone();

        settings.coins = coins;
        settings.ownedRewards = owned_rewards;

        tracker.data.settings = settings.clone();
        tracker.persist();
    }

    let manager = app.autolaunch();

    if settings.autostartEnabled {
        let _ = manager.enable();
    } else {
        let _ = manager.disable();
    }

    let data = {
        let mut tracker = arc.lock().unwrap();
        tracker.current_data()
    };

    data
}

#[tauri::command]
fn save_category(
    state: State<AppState>,
    exe: String,
    category: String,
) -> TrackerData {
    let mut tracker = state.0.lock().unwrap();

    let parsed = match category.as_str() {
        "Productive" => Category::Productive,
        "Distracting" => Category::Distracting,
        "Communication" => Category::Communication,
        "Media" => Category::Media,
        _ => Category::Neutral,
    };

    let key = normalize_exe_target(&exe);

    tracker
        .data
        .settings
        .categoryOverrides
        .insert(key.clone(), parsed.clone());

    if let Some(active) = tracker.active.as_mut() {
        if normalize_exe_target(&active.exe) == key {
            active.category = parsed.clone();
            tracker.data.current_category = Some(parsed);
        }
    }

    tracker.persist();
    tracker.current_data()
}

#[tauri::command]
fn set_tracking_paused(
    state: State<AppState>,
    paused: bool,
) -> TrackerData {
    let mut tracker = state.0.lock().unwrap();

    if paused {
        tracker.finish_active();
        tracker.data.tracking_paused = true;
        tracker.data.current_app = None;
        tracker.data.current_exe = None;
        tracker.data.current_category = None;
        tracker.data.current_session_secs = 0;
        tracker.active = None;
    } else {
        tracker.data.tracking_paused = false;
        tracker.refresh_foreground();
    }

    tracker.persist();
    tracker.current_data()
}

#[tauri::command]
fn complete_focus_session(
    _app: AppHandle,
    state: State<AppState>,
    minutes: u32,
    mode: String,
) -> Result<serde_json::Value, String> {
    let mut tracker = state
        .0
        .lock()
        .map_err(|_| "tracker lock failed")?;

    let safe_minutes = minutes.clamp(5, 180);
    let recent_streak =
        tracker.data.focus_sessions.iter().rev().take(7).count() as u32;

    let points = safe_minutes;
    let coins = safe_minutes
        .saturating_mul(2)
        .saturating_add(recent_streak.saturating_mul(5));

    let mode_enum = match mode.as_str() {
        "Deep Focus" => AppMode::DeepFocus,
        "Casual" => AppMode::Casual,
        _ => AppMode::Smart,
    };

    tracker.data.focus_sessions.push(FocusSession {
        minutes: safe_minutes,
        points,
        coins,
        completed_at: Utc::now().to_rfc3339(),
        mode: mode_enum,
    });

    tracker.data.settings.coins =
        tracker.data.settings.coins.saturating_add(coins);

    if tracker.data.focus_sessions.len() > 1000 {
        let drain = tracker.data.focus_sessions.len() - 1000;
        tracker.data.focus_sessions.drain(0..drain);
    }

    tracker.persist();

    let data = tracker.current_data();

    Ok(serde_json::json!({
        "points": points,
        "coins": coins,
        "data": data
    }))
}

#[tauri::command]
fn set_focus_protection(
    _app: AppHandle,
    state: State<AppState>,
    active: bool,
    blockedApps: Vec<String>,
    mode: String,
) -> TrackerData {
    let mut tracker = state.0.lock().unwrap();

    tracker.focus_active = active;

    tracker.focus_mode = if active {
        Some(match mode.as_str() {
            "Deep Focus" => AppMode::DeepFocus,
            "Casual" => AppMode::Casual,
            _ => AppMode::Smart,
        })
    } else {
        None
    };

    tracker.focus_started =
        if active { Some(Utc::now()) } else { None };

    tracker.focus_apps = blockedApps
        .into_iter()
        .map(|x| normalize_exe_target(&x))
        .filter(|x| safe_exe(x))
        .collect();

    tracker.casual_focus_started.clear();

    tracker.data.settings.deepFocusBlockedApps =
        tracker.focus_apps.iter().cloned().collect();

    tracker.persist();

    if active {
        tracker.refresh_foreground();
        tracker.enforce_active();
    } else {
        hide_limit_overlay();
    }

    tracker.current_data()
}

#[tauri::command]
fn force_close_policy_target(
    state: State<AppState>,
    pid: u32,
    exe: String,
) -> Result<(), String> {
    let mut tracker = state
        .0
        .lock()
        .map_err(|_| "tracker lock failed")?;

    let exe = normalize_exe_target(&exe);

    if !safe_exe(&exe) {
        return Err(
            "That process is not an allowed protection target.".into()
        );
    }

    terminate_process_tree(pid, &exe);
    tracker.close_matching_exe(
        &exe,
        "Closed by Liba Bilancia warning",
    );

    tracker.warning_active = None;
    hide_limit_overlay();

    Ok(())
}

#[tauri::command]
fn close_policy_overlay() -> Result<(), String> {
    hide_limit_overlay();
    Ok(())
}

#[tauri::command]
async fn save_app_limit(
    state: State<'_, AppState>,
    exe: String,
    limit: AppLimit,
) -> Result<TrackerData, String> {
    let key = normalize_exe_target(&exe);

    if !safe_exe(&key) {
        return Err(
            "That executable is not allowed as a protection target.".into()
        );
    }

    if limit.scope == LimitScope::UntilDate {
        let Some(expires) = limit.expiresAt.as_deref() else {
            return Err(
                "Choose an expiry date for this protection.".into()
            );
        };

        let expires = DateTime::parse_from_rfc3339(expires)
            .map_err(|_| "Invalid expiry date.")?;

        if expires.with_timezone(&Utc) <= Utc::now() {
            return Err(
                "The expiry date must be in the future.".into()
            );
        }
    }

    let shared = state.0.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<TrackerData, String> {
        let mut tracker = shared
            .lock()
            .map_err(|_| "tracker lock failed".to_string())?;

        let normalized = AppLimit {
            dailyMinutes: limit.dailyMinutes.max(1),
            action: limit.action,
            enabled: limit.enabled,
            scope: limit.scope,
            startedAt: Some(Utc::now().to_rfc3339()),
            expiresAt: limit.expiresAt,
        };

        tracker
            .data
            .settings
            .appLimits
            .retain(|existing, _| normalize_exe_target(existing) != key);

        tracker
            .data
            .settings
            .appLimits
            .insert(key.clone(), normalized);

        tracker.warn_observed.remove(&key);
        tracker.warn_reopen_counts.remove(&key);
        tracker.close_triggered.remove(&key);
        tracker.warning_active = None;

        // The background worker applies the changed rule. Do not scan processes
        // or close apps inside this UI command.
        tracker.persist();
        Ok(tracker.current_data())
    })
    .await
    .map_err(|_| "The app-protection update worker failed.".to_string())?
}

#[tauri::command]
async fn remove_app_limit(
    state: State<'_, AppState>,
    exe: String,
) -> Result<TrackerData, String> {
    let shared = state.0.clone();
    let key = normalize_exe_target(&exe);

    tauri::async_runtime::spawn_blocking(move || -> Result<TrackerData, String> {
        let mut tracker = shared
            .lock()
            .map_err(|_| "The tracker data lock is unavailable.".to_string())?;

        let existing = tracker
            .data
            .settings
            .appLimits
            .keys()
            .find(|k| normalize_exe_target(k) == key)
            .cloned();

        if let Some(existing) = existing {
            tracker.data.settings.appLimits.remove(&existing);
        }

        tracker.close_triggered.remove(&key);
        tracker.warn_observed.remove(&key);
        tracker.warn_reopen_counts.remove(&key);

        // Removing a rule is a settings-only operation. Enforcement remains on
        // the background worker and is not run while the UI is waiting.
        tracker.persist();
        Ok(tracker.current_data())
    })
    .await
    .map_err(|_| "The app-protection removal worker failed.".to_string())?
}

#[tauri::command]
fn purchase_reward(
    state: State<AppState>,
    rewardId: String,
) -> Result<TrackerData, String> {
    let price = reward_price(&rewardId)
        .ok_or_else(|| "Unknown reward.".to_string())?;

    let mut tracker = state.0.lock().unwrap();

    if tracker
        .data
        .settings
        .ownedRewards
        .iter()
        .any(|x| x == &rewardId)
    {
        return Ok(tracker.current_data());
    }

    if tracker.data.settings.coins < price {
        return Err("You need more Balance Coins.".into());
    }

    tracker.data.settings.coins -= price;
    tracker.data.settings.ownedRewards.push(rewardId.clone());

    if rewardId.starts_with("theme:") {
        tracker
            .data
            .settings
            .purchasedThemes
            .push(rewardId.clone());
    }

    tracker.persist();
    Ok(tracker.current_data())
}

#[tauri::command]
fn equip_reward(
    state: State<AppState>,
    rewardId: String,
) -> Result<TrackerData, String> {
    if reward_price(&rewardId).is_none() {
        return Err("Unknown reward.".into());
    }

    let mut tracker = state.0.lock().unwrap();

    if !tracker
        .data
        .settings
        .ownedRewards
        .contains(&rewardId)
    {
        return Err("Unlock this reward first.".into());
    }

    if rewardId.starts_with("theme:") {
        tracker.data.settings.theme = rewardId.clone();
    } else if rewardId.starts_with("frame:") {
        tracker.data.settings.equippedFrame = rewardId.clone();
    } else if rewardId.starts_with("effect:") {
        tracker.data.settings.equippedEffect = rewardId.clone();
    }

    tracker.persist();
    Ok(tracker.current_data())
}

fn wellbeing_progress(
    active: &ActiveWellbeing,
    duration: u64,
) -> u8 {
    let elapsed = active.started.elapsed().as_secs();

    ((elapsed.saturating_mul(100) / duration.max(1)).min(100)) as u8
}

#[tauri::command]
fn get_wellbeing_records(
    state: State<AppState>,
) -> Vec<WellbeingRecord> {
    state.0.lock().unwrap().wellbeing_records.clone()
}

#[tauri::command]
fn start_wellbeing_activity(
    state: State<AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let duration = wellbeing_duration_secs(&id)
        .ok_or_else(|| "Unknown wellbeing activity.".to_string())?;

    let mut tracker = state
        .0
        .lock()
        .map_err(|_| "tracker lock failed")?;

    let today = wellbeing_day_records(
        &tracker.wellbeing_records,
    );

    if today.len() >= 3 {
        return Err(
            "Daily wellbeing reward limit reached.".into()
        );
    }

    if today.iter().filter(|r| r.id == id).count() >= 2 {
        return Err(
            "You have already completed this activity twice today."
                .into(),
        );
    }

    if tracker.wellbeing_active.is_some() {
        return Err(
            "A wellbeing activity is already running.".into()
        );
    }

    tracker.wellbeing_active = Some(ActiveWellbeing {
        id: id.clone(),
        started: Instant::now(),
        completed_steps: 0,
    });

    Ok(serde_json::json!({
        "id": id,
        "durationSecs": duration,
        "completedSteps": 0,
        "progress": 0
    }))
}

#[tauri::command]
fn advance_wellbeing_step(
    state: State<AppState>,
) -> Result<serde_json::Value, String> {
    let mut tracker = state
        .0
        .lock()
        .map_err(|_| "tracker lock failed")?;

    let Some(active) = tracker.wellbeing_active.as_mut() else {
        return Err(
            "No wellbeing activity is running.".into()
        );
    };

    let duration =
        wellbeing_duration_secs(&active.id).unwrap_or(180);

    let progress = {
        let elapsed = active.started.elapsed().as_secs();
        ((elapsed.saturating_mul(100) / duration.max(1)).min(100))
            as u8
    };

    let required = match active.completed_steps {
        0 => 25,
        1 => 50,
        2 => 75,
        _ => 100,
    };

    if progress < required {
        return Err(format!(
            "Complete at least {}% of the activity before continuing.",
            required
        ));
    }

    if active.completed_steps < 3 {
        active.completed_steps += 1;
    }

    Ok(serde_json::json!({
        "id": active.id,
        "durationSecs": duration,
        "completedSteps": active.completed_steps,
        "progress": progress
    }))
}

#[tauri::command]
fn cancel_wellbeing_activity(
    state: State<AppState>,
) -> Result<(), String> {
    let mut tracker = state
        .0
        .lock()
        .map_err(|_| "tracker lock failed")?;

    tracker.wellbeing_active = None;
    Ok(())
}

#[tauri::command]
fn complete_wellbeing_activity(
    state: State<AppState>,
) -> Result<serde_json::Value, String> {
    let mut tracker = state
        .0
        .lock()
        .map_err(|_| "tracker lock failed")?;

    let Some(active) = tracker.wellbeing_active.clone() else {
        return Err(
            "No wellbeing activity is running.".into()
        );
    };

    let duration =
        wellbeing_duration_secs(&active.id).unwrap_or(180);

    let elapsed = active.started.elapsed().as_secs();
    let progress =
        ((elapsed.saturating_mul(100) / duration.max(1)).min(100))
            as u8;

    if progress < 100 || active.completed_steps < 3 {
        return Err(
            "Finish the full activity and all steps first.".into()
        );
    }

    let today =
        wellbeing_day_records(&tracker.wellbeing_records);

    if today.len() >= 3 {
        tracker.wellbeing_active = None;
        return Err(
            "Daily wellbeing reward limit reached.".into()
        );
    }

    if today.iter().filter(|r| r.id == active.id).count() >= 2 {
        tracker.wellbeing_active = None;
        return Err(
            "This activity has already been rewarded twice today."
                .into(),
        );
    }

    let coins = wellbeing_coins(duration);

    let record = WellbeingRecord {
        id: active.id.clone(),
        completed_at: Utc::now().to_rfc3339(),
        duration_secs: duration,
        coins,
    };

    tracker.wellbeing_records.push(record.clone());
    tracker.data.settings.coins =
        tracker.data.settings.coins.saturating_add(coins);
    tracker.wellbeing_active = None;

    if tracker.wellbeing_records.len() > 1000 {
        let drain =
            tracker.wellbeing_records.len() - 1000;
        tracker.wellbeing_records.drain(0..drain);
    }

    tracker.persist();

    Ok(serde_json::json!({
        "record": record,
        "coins": coins,
        "data": tracker.current_data()
    }))
}

#[tauri::command]
fn delete_local_data(
    state: State<AppState>,
) -> TrackerData {
    let mut tracker = state.0.lock().unwrap();

    tracker.data.sessions.clear();
    tracker.data.focus_sessions.clear();
    tracker.data.settings.appLimits.clear();
    tracker.wellbeing_records.clear();
    tracker.wellbeing_active = None;
    tracker.close_triggered.clear();
    tracker.warn_observed.clear();
    tracker.warn_reopen_counts.clear();

    tracker.data.current_app = None;
    tracker.data.current_exe = None;
    tracker.data.current_category = None;
    tracker.data.current_session_secs = 0;

    tracker.persist();
    tracker.current_data()
}

#[tauri::command]
fn export_local_data(
    state: State<AppState>,
) -> String {
    serde_json::to_string_pretty(
        &state.0.lock().unwrap().data,
    )
    .unwrap_or_else(|_| "{}".into())
}

#[tauri::command]
fn hide_to_tray(
    window: tauri::WebviewWindow,
) -> Result<(), String> {
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
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            #[cfg(windows)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register_all();
            }

            let open_item =
                MenuItemBuilder::with_id(
                    "open",
                    "Open Liba Bilancia",
                )
                .build(app)?;

            let quit_item =
                MenuItemBuilder::with_id(
                    "quit",
                    "Quit Liba Bilancia",
                )
                .build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&open_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .expect(
                    "Liba Bilancia default icon is required for the system tray",
                );

            TrayIconBuilder::with_id(
                "digital-balance-tray",
            )
            .icon(tray_icon)
            .tooltip(
                "Liba Bilancia - running in the background",
            )
            .menu(&tray_menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "open" => {
                    if let Some(window) =
                        app.get_webview_window("main")
                    {
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
                } = event
                {
                    let app = tray.app_handle();

                    if let Some(window) =
                        app.get_webview_window("main")
                    {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            })
            .build(app)?;

            let path =
                app.path().app_data_dir()?.join(
                    "digital-balance.json",
                );

            let tracker = Tracker::load(path);
            let shared = Arc::new(Mutex::new(tracker));

            app.manage(AppState(shared.clone()));

            let _ = GLOBAL_TRACKER.set(shared.clone());
            let _ = GLOBAL_APP.set(app.handle().clone());

            #[cfg(windows)]
            {
                thread::spawn(start_windows_hook);
            }

            thread::spawn(|| loop {
                thread::sleep(Duration::from_millis(1000));

                if let Some(shared) =
                    GLOBAL_TRACKER.get()
                {
                    // Never wait behind a UI command or another tracker task.
                    // If the lock is busy, the next one-second pass will catch up.
                    if let Ok(mut tracker) =
                        shared.try_lock()
                    {
                        tracker.refresh_foreground();
                        tracker.enforce_active();
                    }
                }
            });

            Ok(())
        });

    builder
        .invoke_handler(tauri::generate_handler![
            get_tracker_data,
            get_running_apps,
            get_protection_apps,
            save_settings,
            save_category,
            set_tracking_paused,
            complete_focus_session,
            set_focus_protection,
            force_close_policy_target,
            close_policy_overlay,
            save_app_limit,
            remove_app_limit,
            purchase_reward,
            equip_reward,
            get_wellbeing_records,
            start_wellbeing_activity,
            advance_wellbeing_step,
            cancel_wellbeing_activity,
            complete_wellbeing_activity,
            delete_local_data,
            export_local_data,
            hide_to_tray,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Liba Bilancia");
}
