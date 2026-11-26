param(
    [string]$ProjectRoot = (Get-Location).Path
)

$ErrorActionPreference = "Stop"

$libPath = Join-Path $ProjectRoot "src-tauri\src\lib.rs"
if (-not (Test-Path -LiteralPath $libPath)) {
    throw "Could not find src-tauri\src\lib.rs under $ProjectRoot. No files were written."
}

$source = (Get-Content -LiteralPath $libPath -Raw) -replace "`r`n", "`n" -replace "`r", "`n"

function Require-One([string]$Label, [string]$Text) {
    if ([string]::IsNullOrEmpty($Text)) {
        throw "The '$Label' anchor text is empty. No files have been written."
    }
    $count = ([regex]::Matches($script:source, [regex]::Escape($Text))).Count
    if ($count -ne 1) {
        throw "Expected exactly one '$Label' anchor, found $count. No files have been written."
    }
}

function Replace-One([string]$Label, [string]$Old, [string]$New) {
    Require-One $Label $Old
    $script:source = $script:source.Replace($Old, $New)
}

# ---- Preflight: exact anchors from the user's current lib.rs ----
$checks = @(
    @("safe_exe", "fn safe_exe(exe: &str) -> bool {"),
    @("LimitAction derive", "#[derive(Debug, Clone, Serialize, Deserialize)]`n#[serde(rename_all = `"PascalCase`")]`npub enum LimitAction { Warn, Close, Block }"),
    @("Tracker struct", "struct Tracker { data: TrackerData, path: PathBuf, active: Option<ActiveSession>, focus_active: bool, focus_mode: Option<AppMode>, focus_started: Option<DateTime<Utc>>, focus_apps: HashSet<String>, casual_focus_started: HashMap<String, DateTime<Utc>>, warn_observed: HashSet<String>, warn_reopen_counts: HashMap<String, u8>, warning_active: Option<(String, u32)>, closed_date: String }"),
    @("RunningApp struct", "struct RunningApp { name: String, exe: String }"),
    @("save_settings body", "{ let mut tracker = arc.lock().unwrap(); tracker.data.settings = settings.clone(); tracker.persist(); }"),
    @("poll interval", "thread::sleep(Duration::from_secs(1));"),
    @("invoke handler tail", "equip_reward, delete_local_data, export_local_data, hide_to_tray, quit_app")
)
foreach ($c in $checks) { Require-One $c[0] $c[1] }

# ---- 1. Imports / derives / state ----
Replace-One "time import" `
'use std::{collections::{HashMap, HashSet}, fs, path::PathBuf, sync::{Arc, Mutex}, thread, time::Duration};' `
'use std::{collections::{HashMap, HashSet}, fs, path::PathBuf, sync::{Arc, Mutex}, thread, time::{Duration, Instant}};'

Replace-One "LimitAction derive" `
'#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LimitAction { Warn, Close, Block }' `
'#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LimitAction { Warn, Close, Block }'

Replace-One "LimitScope derive" `
'#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitScope { Daily, UntilDate }' `
'#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitScope { Daily, UntilDate }'

$wellbeingStructs = @'
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

'@
Replace-One "Tracker struct" `
'struct Tracker { data: TrackerData, path: PathBuf, active: Option<ActiveSession>, focus_active: bool, focus_mode: Option<AppMode>, focus_started: Option<DateTime<Utc>>, focus_apps: HashSet<String>, casual_focus_started: HashMap<String, DateTime<Utc>>, warn_observed: HashSet<String>, warn_reopen_counts: HashMap<String, u8>, warning_active: Option<(String, u32)>, closed_date: String }' `
'struct Tracker { data: TrackerData, path: PathBuf, active: Option<ActiveSession>, focus_active: bool, focus_mode: Option<AppMode>, focus_started: Option<DateTime<Utc>>, focus_apps: HashSet<String>, casual_focus_started: HashMap<String, DateTime<Utc>>, warn_observed: HashSet<String>, warn_reopen_counts: HashMap<String, u8>, warning_active: Option<(String, u32)>, close_triggered: HashSet<String>, wellbeing_records: Vec<WellbeingRecord>, wellbeing_active: Option<ActiveWellbeing>, closed_date: String }'

# Insert wellbeing structs using a position-based insertion instead of a brittle
# multiline replacement. This avoids CRLF/PowerShell here-string matching issues.
$wellbeingAnchor = "fn local_day() -> String"
if ($source.Contains("struct ActiveWellbeing")) {
    # Already present; do nothing.
} else {
    $idx = $source.IndexOf($wellbeingAnchor, [StringComparison]::Ordinal)
    if ($idx -lt 0) {
        throw "Could not locate the local_day insertion point. No files have been written."
    }
    $source = $source.Insert($idx, $wellbeingStructs + "`n")
}

# ---- 2. safe_exe: require real .exe and retain system exclusions ----
$oldSafe = @'
fn safe_exe(exe: &str) -> bool {
    let lower = exe.to_ascii_lowercase();
    if exe.is_empty() || exe.len() > 160 { return false; }
    if !exe.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')) { return false; }
    !matches!(lower.as_str(),
'@
$newSafe = @'
fn safe_exe(exe: &str) -> bool {
    let lower = exe.to_ascii_lowercase();
    if exe.is_empty() || exe.len() > 160 { return false; }
    if !lower.ends_with(".exe") { return false; }
    if !exe.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')) { return false; }
    !matches!(lower.as_str(),
'@
Replace-One "safe_exe" $oldSafe $newSafe

# ---- 3. helpers ----
Replace-One "free theme boundary" `
'fn free_theme_ids() -> Vec<String> {' `
'fn normalize_exe_target(exe: &str) -> String {
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
    records.iter()
        .filter(|r| r.completed_at.get(..10).unwrap_or("") == today)
        .cloned()
        .collect()
}

fn free_theme_ids() -> Vec<String> {'

# ---- 4. Tracker::load + daily reset ----
Replace-One "Tracker load state" `
'Self { data, path, active: None, focus_active: false, focus_mode: None, focus_started: None, focus_apps: HashSet::new(), casual_focus_started: HashMap::new(), warn_observed: HashSet::new(), warn_reopen_counts: HashMap::new(), warning_active: None, closed_date: local_day() }' `
'Self { data, path, active: None, focus_active: false, focus_mode: None, focus_started: None, focus_apps: HashSet::new(), casual_focus_started: HashMap::new(), warn_observed: HashSet::new(), warn_reopen_counts: HashMap::new(), warning_active: None, close_triggered: HashSet::new(), wellbeing_records: Vec::new(), wellbeing_active: None, closed_date: local_day() }'

Replace-One "daily reset" `
'fn normalize_day(&mut self) { let now = local_day(); if now != self.closed_date { self.closed_date = now; self.warn_observed.clear(); self.warn_reopen_counts.clear(); self.warning_active = None; hide_limit_overlay(); } }' `
'fn normalize_day(&mut self) { let now = local_day(); if now != self.closed_date { self.closed_date = now; self.warn_observed.clear(); self.warn_reopen_counts.clear(); self.close_triggered.clear(); self.warning_active = None; hide_limit_overlay(); } }'

# ---- 5. settings preservation ----
Replace-One "save_settings body" `
'{ let mut tracker = arc.lock().unwrap(); tracker.data.settings = settings.clone(); tracker.persist(); }' `
'{ let mut tracker = arc.lock().unwrap();
        let coins = tracker.data.settings.coins;
        let owned_rewards = tracker.data.settings.ownedRewards.clone();
        settings.coins = coins;
        settings.ownedRewards = owned_rewards;
        tracker.data.settings = settings.clone();
        tracker.persist();
    }'

# ---- 6. installed flag + app scanning ----
Replace-One "RunningApp struct" `
'struct RunningApp { name: String, exe: String }' `
'struct RunningApp { name: String, exe: String, installed: bool }'

Replace-One "running app constructor" `
'RunningApp { name: friendly_process_name(&exe), exe }' `
'RunningApp { name: friendly_process_name(&exe), exe, installed: false }'

Replace-One "installed app constructor" `
'RunningApp { name: if name.trim().is_empty() { friendly_process_name(exe) } else { name.to_string() }, exe: exe.to_string() }' `
'RunningApp { name: if name.trim().is_empty() { friendly_process_name(exe) } else { name.to_string() }, exe: exe.to_string(), installed: true }'

Replace-One "installed roots (user)" `
'"$env:APPDATA\Microsoft\Windows\Start Menu\Programs"' `
'"$env:APPDATA\Microsoft\Windows\Start Menu\Programs",
  "$env:USERPROFILE\Desktop"'

Replace-One "installed roots (public)" `
'"$env:ProgramData\Microsoft\Windows\Start Menu\Programs"' `
'"$env:ProgramData\Microsoft\Windows\Start Menu\Programs",
  "$env:PUBLIC\Desktop"'

$fallbackOld = @'
RunningApp { name: "Google Chrome".into(), exe: "chrome.exe".into() },
        RunningApp { name: "Microsoft Edge".into(), exe: "msedge.exe".into() },
        RunningApp { name: "Mozilla Firefox".into(), exe: "firefox.exe".into() },
        RunningApp { name: "Discord".into(), exe: "discord.exe".into() },
        RunningApp { name: "Spotify".into(), exe: "spotify.exe".into() },
        RunningApp { name: "Steam".into(), exe: "steam.exe".into() },
        RunningApp { name: "Telegram".into(), exe: "telegram.exe".into() },
        RunningApp { name: "VLC media player".into(), exe: "vlc.exe".into() },
'@
$fallbackNew = @'
RunningApp { name: "Google Chrome".into(), exe: "chrome.exe".into(), installed: false },
        RunningApp { name: "Microsoft Edge".into(), exe: "msedge.exe".into(), installed: false },
        RunningApp { name: "Mozilla Firefox".into(), exe: "firefox.exe".into(), installed: false },
        RunningApp { name: "Discord".into(), exe: "discord.exe".into(), installed: false },
        RunningApp { name: "Spotify".into(), exe: "spotify.exe".into(), installed: false },
        RunningApp { name: "Steam".into(), exe: "steam.exe".into(), installed: false },
        RunningApp { name: "Telegram".into(), exe: "telegram.exe".into(), installed: false },
        RunningApp { name: "VLC media player".into(), exe: "vlc.exe".into(), installed: false },
'@
Replace-One "fallback apps" $fallbackOld $fallbackNew

# ---- 7. non-Windows visible process stub ----
Replace-One "visible process cfg boundary" `
'#[cfg(windows)]
fn running_apps_for_windows()' `
'#[cfg(not(windows))]
fn enumerate_visible_process_ids() -> HashSet<u32> { HashSet::new() }

#[cfg(windows)]
fn running_apps_for_windows()'

# ---- 8. enforcement: visible-window Warn, Close one-shot, Block repeated ----
$oldEnforce = @'
        let processes = enumerate_processes();
        let running_exes: HashSet<String> = processes.iter().map(|p| p.exe.to_ascii_lowercase()).collect();
        self.warn_observed.retain(|key| running_exes.contains(key));

        if let Some((warning_exe, warning_pid)) = self.warning_active.clone() {
            if !running_exes.contains(&warning_exe) || !process_exists(warning_pid) {
'@
$newEnforce = @'
        let processes = enumerate_processes();
        let running_exes: HashSet<String> = processes.iter().map(|p| p.exe.to_ascii_lowercase()).collect();
        let visible_pids = enumerate_visible_process_ids();
        let visible_exes: HashSet<String> = processes.iter()
            .filter(|p| visible_pids.contains(&p.pid))
            .map(|p| p.exe.to_ascii_lowercase())
            .collect();
        self.warn_observed.retain(|key| visible_exes.contains(key));

        if let Some((warning_exe, warning_pid)) = self.warning_active.clone() {
            if !visible_exes.contains(&warning_exe) || !process_exists(warning_pid) {
'@
Replace-One "visible warn state" $oldEnforce $newEnforce

Replace-One "warning matches" `
'let matches: Vec<ProcessInfo> = processes.iter().filter(|p| p.exe.eq_ignore_ascii_case(&configured_exe)).cloned().collect();
            if matches.is_empty() { continue; }' `
'let matches: Vec<ProcessInfo> = processes.iter().filter(|p| p.exe.eq_ignore_ascii_case(&configured_exe)).cloned().collect();
            if matches.is_empty() { continue; }
            let visible_matches: Vec<ProcessInfo> = matches.iter().filter(|p| visible_pids.contains(&p.pid)).cloned().collect();'

Replace-One "warn branch" `
'if !self.warn_observed.contains(&key) {' `
'if !self.warn_observed.contains(&key) && !visible_matches.is_empty() {'

Replace-One "warn pid" `
'let pid = matches[0].pid;' `
'let pid = visible_matches[0].pid;'

$closeOld = @'
                LimitAction::Close => {
                    self.close_matching_exe(&configured_exe, "Limit reached — app closed");
                }
                LimitAction::Block => {
                    self.close_matching_exe(&configured_exe, "Limit reached — app blocked until the rule resets");
                }
'@
$closeNew = @'
                LimitAction::Close => {
                    if !self.close_triggered.contains(&key) {
                        self.close_triggered.insert(key.clone());
                        self.close_matching_exe(&configured_exe, "Limit reached - app closed");
                    }
                }
                LimitAction::Block => {
                    self.close_matching_exe(&configured_exe, "Limit reached - app blocked until the rule resets");
                }
'@
Replace-One "Close/Block behavior" $closeOld $closeNew

# ---- 9. canonical app-limit keys + reset close state ----
Replace-One "save app limit insert" `
'tracker.data.settings.appLimits.insert(exe.clone(), normalized);
    tracker.warn_observed.remove(&exe.to_ascii_lowercase());
    tracker.warn_reopen_counts.remove(&exe.to_ascii_lowercase());' `
'let key = normalize_exe_target(&exe);
    tracker.data.settings.appLimits.retain(|existing, _| normalize_exe_target(existing) != key);
    tracker.data.settings.appLimits.insert(key.clone(), normalized);
    tracker.warn_observed.remove(&key);
    tracker.warn_reopen_counts.remove(&key);
    tracker.close_triggered.remove(&key);'

Replace-One "remove app limit" `
'fn remove_app_limit(state: State<AppState>, exe: String) -> TrackerData { let mut tracker=state.0.lock().unwrap(); tracker.data.settings.appLimits.remove(&exe); tracker.persist(); tracker.current_data() }' `
'fn remove_app_limit(state: State<AppState>, exe: String) -> TrackerData {
    let mut tracker=state.0.lock().unwrap();
    let key = normalize_exe_target(&exe);
    let existing = tracker.data.settings.appLimits.keys().find(|k| normalize_exe_target(k) == key).cloned();
    if let Some(existing) = existing { tracker.data.settings.appLimits.remove(&existing); }
    tracker.close_triggered.remove(&key);
    tracker.warn_observed.remove(&key);
    tracker.warn_reopen_counts.remove(&key);
    tracker.persist();
    tracker.current_data()
}'

# ---- 10. Focus canonicalization ----
Replace-One "focus app normalization" `
'tracker.focus_apps = blockedApps.into_iter().map(|x| x.to_ascii_lowercase()).filter(|x| safe_exe(x)).collect();' `
'tracker.focus_apps = blockedApps.into_iter().map(|x| normalize_exe_target(&x)).filter(|x| safe_exe(x)).collect();'

# ---- 11. wellbeing commands ----
$wellbeingFns = @'
fn wellbeing_progress(active: &ActiveWellbeing, duration: u64) -> u8 {
    let elapsed = active.started.elapsed().as_secs();
    ((elapsed.saturating_mul(100) / duration.max(1)).min(100)) as u8
}

#[tauri::command]
fn get_wellbeing_records(state: State<AppState>) -> Vec<WellbeingRecord> {
    state.0.lock().unwrap().wellbeing_records.clone()
}

#[tauri::command]
fn start_wellbeing_activity(state: State<AppState>, id: String) -> Result<serde_json::Value, String> {
    let duration = wellbeing_duration_secs(&id).ok_or_else(|| "Unknown wellbeing activity.".to_string())?;
    let mut tracker = state.0.lock().map_err(|_| "tracker lock failed")?;
    let today = wellbeing_day_records(&tracker.wellbeing_records);
    if today.len() >= 3 { return Err("Daily wellbeing reward limit reached.".into()); }
    if today.iter().filter(|r| r.id == id).count() >= 2 { return Err("You have already completed this activity twice today.".into()); }
    if tracker.wellbeing_active.is_some() { return Err("A wellbeing activity is already running.".into()); }
    tracker.wellbeing_active = Some(ActiveWellbeing { id: id.clone(), started: Instant::now(), completed_steps: 0 });
    Ok(serde_json::json!({"id":id,"durationSecs":duration,"completedSteps":0,"progress":0}))
}

#[tauri::command]
fn advance_wellbeing_step(state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut tracker = state.0.lock().map_err(|_| "tracker lock failed")?;
    let Some(active) = tracker.wellbeing_active.as_mut() else { return Err("No wellbeing activity is running.".into()); };
    let duration = wellbeing_duration_secs(&active.id).unwrap_or(180);
    let progress = wellbeing_progress(active, duration);
    let required = match active.completed_steps { 0 => 25, 1 => 50, 2 => 75, _ => 100 };
    if progress < required { return Err(format!("Complete at least {}% of the activity before continuing.", required)); }
    if active.completed_steps < 3 { active.completed_steps += 1; }
    Ok(serde_json::json!({"id":active.id,"durationSecs":duration,"completedSteps":active.completed_steps,"progress":progress}))
}

#[tauri::command]
fn cancel_wellbeing_activity(state: State<AppState>) -> Result<(), String> {
    let mut tracker = state.0.lock().map_err(|_| "tracker lock failed")?;
    tracker.wellbeing_active = None;
    Ok(())
}

#[tauri::command]
fn complete_wellbeing_activity(state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut tracker = state.0.lock().map_err(|_| "tracker lock failed")?;
    let Some(active) = tracker.wellbeing_active.clone() else { return Err("No wellbeing activity is running.".into()); };
    let duration = wellbeing_duration_secs(&active.id).unwrap_or(180);
    let progress = wellbeing_progress(&active, duration);
    if progress < 100 || active.completed_steps < 3 { return Err("Finish the full activity and all steps first.".into()); }
    let today = wellbeing_day_records(&tracker.wellbeing_records);
    if today.len() >= 3 { tracker.wellbeing_active = None; return Err("Daily wellbeing reward limit reached.".into()); }
    if today.iter().filter(|r| r.id == active.id).count() >= 2 { tracker.wellbeing_active = None; return Err("This activity has already been rewarded twice today.".into()); }
    let coins = wellbeing_coins(duration);
    let record = WellbeingRecord { id: active.id.clone(), completed_at: Utc::now().to_rfc3339(), duration_secs: duration, coins };
    tracker.wellbeing_records.push(record.clone());
    tracker.data.settings.coins = tracker.data.settings.coins.saturating_add(coins);
    tracker.wellbeing_active = None;
    if tracker.wellbeing_records.len() > 1000 { let drain = tracker.wellbeing_records.len() - 1000; tracker.wellbeing_records.drain(0..drain); }
    tracker.persist();
    Ok(serde_json::json!({"record":record,"coins":coins,"data":tracker.current_data()}))
}

'@
Replace-One "wellbeing command insertion" `
'#[tauri::command]
fn delete_local_data' `
"$wellbeingFns#[tauri::command]
fn delete_local_data"

# ---- 12. command registration + polling ----
Replace-One "wellbeing invoke registration" `
'equip_reward, delete_local_data, export_local_data, hide_to_tray, quit_app' `
'equip_reward, get_wellbeing_records, start_wellbeing_activity, advance_wellbeing_step, cancel_wellbeing_activity, complete_wellbeing_activity,
        delete_local_data, export_local_data, hide_to_tray, quit_app'

Replace-One "poll interval" `
'thread::sleep(Duration::from_secs(1));' `
'thread::sleep(Duration::from_millis(500));'

# ---- 13. Validate key results before writing ----
$requiredSnippets = @(
    "fn safe_exe(exe: &str) -> bool",
    "if !lower.ends_with(`".exe`")",
    "struct ActiveWellbeing {`n    id: String,`n    started: Instant,`n    completed_steps: u8,`n}",
    "close_triggered: HashSet<String>",
    "installed: bool",
    "fn get_wellbeing_records",
    "fn start_wellbeing_activity",
    "fn advance_wellbeing_step",
    "fn complete_wellbeing_activity",
    "thread::sleep(Duration::from_millis(500));"
)
foreach ($snippet in $requiredSnippets) {
    if (-not $source.Contains($snippet)) {
        throw "Post-patch validation failed for '$snippet'. No files have been written."
    }
}

# Write only after all validation passes.
Set-Content -LiteralPath $libPath -Value $source -Encoding UTF8
Write-Host ""
Write-Host "V2.2 control-engine patch applied successfully." -ForegroundColor Green
Write-Host "Updated: $libPath"
Write-Host "Next: run npm.cmd run build before changing versions, tags, or release workflows."
