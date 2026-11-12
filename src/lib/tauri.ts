import { invoke } from '@tauri-apps/api/core';
import type { AppLimit, AppSettings, TrackerData } from './types';

export async function getTrackerData() { return invoke<TrackerData>('get_tracker_data'); }
export async function saveSettings(settings: AppSettings) { return invoke<TrackerData>('save_settings', { settings }); }
export async function saveCategory(exe: string, category: string) { return invoke<TrackerData>('save_category', { exe, category }); }
export async function setTrackingPaused(paused: boolean) { return invoke<TrackerData>('set_tracking_paused', { paused }); }
export async function completeFocusSession(minutes: number, mode: string) { return invoke<{ points: number; coins: number; data: TrackerData }>('complete_focus_session', { minutes, mode }); }
export async function setFocusProtection(active: boolean, blockedApps: string[], mode: string) { return invoke<TrackerData>('set_focus_protection', { active, blockedApps, mode }); }
export async function saveAppLimit(exe: string, limit: AppLimit) { return invoke<TrackerData>('save_app_limit', { exe, limit }); }
export async function removeAppLimit(exe: string) { return invoke<TrackerData>('remove_app_limit', { exe }); }
export async function purchaseReward(rewardId: string) { return invoke<TrackerData>('purchase_reward', { rewardId }); }
export async function forceClosePolicyTarget(pid: number, exe: string) { return invoke<void>('force_close_policy_target', { pid, exe }); }
export async function closePolicyOverlay() { return invoke<void>('close_policy_overlay'); }
export async function equipReward(rewardId: string) { return invoke<TrackerData>('equip_reward', { rewardId }); }
export async function deleteLocalData() { return invoke<TrackerData>('delete_local_data'); }
export async function exportLocalData() { return invoke<string>('export_local_data'); }
export async function hideToTray() { return invoke<void>('hide_to_tray'); }
export async function quitApp() { return invoke<void>('quit_app'); }
