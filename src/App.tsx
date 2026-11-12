import { useEffect, useMemo, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import type { Session, User } from '@supabase/supabase-js';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { check, type Update } from '@tauri-apps/plugin-updater';
import {
  awardFocusPoints,
  ensureProfile,
  fetchLeaderboard,
  setupDeepLinkAuth,
  signInWithGoogle,
  supabase,
  supabaseConfigured,
  supabaseConfigIssue,
  updateProfileDisplayName,
} from './lib/supabase';
import {
  completeFocusSession,
  deleteLocalData,
  equipReward,
  exportLocalData,
  getTrackerData,
  hideToTray,
  quitApp,
  purchaseReward,
  removeAppLimit,
  saveAppLimit,
  saveCategory,
  saveSettings,
  setFocusProtection,
  setTrackingPaused,
} from './lib/tauri';
import type { AppLimit, AppMode, AppSettings, Category, LeaderboardRow, LimitAction, LimitScope, Reward, TrackerData } from './lib/types';

const modes: AppMode[] = ['Casual', 'Smart', 'Deep Focus'];
const categories: Category[] = ['Productive', 'Distracting', 'Communication', 'Media', 'Neutral'];
type View = 'Overview' | 'Focus' | 'Insights' | 'Control' | 'Rewards' | 'Leaderboard' | 'Settings';

const PROTECTED_EXES = new Set(['digital-balance.exe', 'explorer.exe', 'system', 'smss.exe', 'csrss.exe', 'wininit.exe', 'winlogon.exe', 'services.exe', 'lsass.exe', 'svchost.exe', 'dwm.exe']);
const AESTHETIC_NAMES = [
  'Midnight Atlas','Velvet Raven','Quiet Nova','Neon Sage','Lunar Fox','Cobalt Drift','Ivory Orbit','Solar Echo','Ghost Signal','Arctic Bloom',
  'Night Shift','Static Bloom','Chrome Vale','Violet Run','Ember Sky','Pixel Tide','Silver Moss','Golden Hour','Aster Nova','Blue Hour',
  'Cosmic Vale','Frost Circuit','Crimson Arc','Glass Harbor','Wild Orbit','Sapphire Mist','Astral Bloom','Cloud Runner','Neon Harbor','Moonline',
  'Deep Current','Prism Wolf','Nova Field','Midnight Bloom','Copper Signal','Aurora Lane','Soft Static','Pulse Atlas','Zero Hour','Velvet Grid',
];
const QUOTES = [
  'You do not need another tab. You need another decision.',
  'Your attention is expensive. Stop giving it away for free.',
  'Another scroll will not finish what you started.',
  'You already know what happens if you open it again. Choose differently.',
  'Busy is not the same thing as productive.',
  'The distraction is asking for five minutes. Your goal is asking for the hour.',
  'You can come back later. Your focus deserves a turn now.',
  'Autopilot is comfortable. Progress usually is not.',
  'You said you wanted better habits. This is the rep that counts.',
  'Stop feeding the loop. Start feeding the goal.',
  'Protect the next ten minutes. They add up.',
  'You are one deliberate click away from getting back on track.',
];

const BASE_THEME_NAMES = [
  'Obsidian','Aurora','Ember','Forest','Ocean','Violet','Rose','Sunset','Cyber','Glass',
  'Nebula','Graphite','Titanium','Arctic','Crimson','Sapphire','Amethyst','Emerald','Ruby','Solar',
  'Lunar','Plasma','Midnight','Storm','Monsoon','Velvet','Cobalt','Magenta','Icefire','Toxic',
  'Quartz','Halo','Nova','Circuit','Horizon','Dusk','Dawn','Mirage','Eclipse','Flux',
  'Lagoon','Boreal','Onyx','Pearl','Copper','Indigo','Jade','Scarlet','Polar','Astral',
];
const THEME_MODES = ['Core','Flux'];
const FREE_THEME_IDS = ['theme:obsidian','theme:aurora','theme:ember','theme:forest','theme:ocean','theme:violet','theme:rose','theme:sunset'];

const THEME_PALETTES: Record<string, { accent: string; accent2: string; bg: string; panel: string; panel2: string; line: string }> = {
  Obsidian: { accent: '#60a5fa', accent2: '#8b5cf6', bg: '#070b12', panel: '#101722', panel2: '#0b111a', line: '#202b39' },
  Aurora: { accent: '#67e8f9', accent2: '#a78bfa', bg: '#071017', panel: '#0d171f', panel2: '#09121a', line: '#1e3340' },
  Ember: { accent: '#fb923c', accent2: '#f43f5e', bg: '#120b08', panel: '#1b110d', panel2: '#130c0a', line: '#352018' },
  Forest: { accent: '#4ade80', accent2: '#22c55e', bg: '#071109', panel: '#0d1810', panel2: '#09130c', line: '#1c3324' },
  Ocean: { accent: '#22d3ee', accent2: '#3b82f6', bg: '#061018', panel: '#0c1823', panel2: '#08121b', line: '#1b3242' },
  Violet: { accent: '#a78bfa', accent2: '#ec4899', bg: '#0d0916', panel: '#161025', panel2: '#100b1b', line: '#302244' },
  Rose: { accent: '#fb7185', accent2: '#f472b6', bg: '#16090e', panel: '#1b0d14', panel2: '#12090f', line: '#3a1c2a' },
  Sunset: { accent: '#f97316', accent2: '#facc15', bg: '#180d07', panel: '#20120b', panel2: '#160d08', line: '#3a2416' },
  Cyber: { accent: '#a3e635', accent2: '#22c55e', bg: '#071007', panel: '#0d180b', panel2: '#091208', line: '#20331b' },
  Glass: { accent: '#334155', accent2: '#38bdf8', bg: '#eaf0f6', panel: 'rgba(255,255,255,.72)', panel2: 'rgba(255,255,255,.54)', line: '#cbd5e1' },
  Nebula: { accent: '#8b5cf6', accent2: '#22d3ee', bg: '#09071a', panel: '#15112a', panel2: '#0f0c20', line: '#2b2450' },
  Graphite: { accent: '#94a3b8', accent2: '#64748b', bg: '#090b0f', panel: '#13171d', panel2: '#0e1217', line: '#252c36' },
  Titanium: { accent: '#cbd5e1', accent2: '#60a5fa', bg: '#0b0f14', panel: '#171d25', panel2: '#11171e', line: '#2b3643' },
  Arctic: { accent: '#7dd3fc', accent2: '#e0f2fe', bg: '#071018', panel: '#0e1a24', panel2: '#0a141d', line: '#213b4e' },
  Crimson: { accent: '#ef4444', accent2: '#f97316', bg: '#160708', panel: '#1f0d10', panel2: '#15090c', line: '#3c1b20' },
  Sapphire: { accent: '#3b82f6', accent2: '#06b6d4', bg: '#070c17', panel: '#0d1625', panel2: '#09111e', line: '#203452' },
  Amethyst: { accent: '#c084fc', accent2: '#8b5cf6', bg: '#110919', panel: '#1a1028', panel2: '#120c1d', line: '#342248' },
  Emerald: { accent: '#34d399', accent2: '#10b981', bg: '#06110d', panel: '#0d1a14', panel2: '#09130f', line: '#1e3a2c' },
  Ruby: { accent: '#f43f5e', accent2: '#fb7185', bg: '#16070c', panel: '#1f0d14', panel2: '#14090f', line: '#3b1b27' },
  Solar: { accent: '#facc15', accent2: '#f97316', bg: '#171008', panel: '#20170b', panel2: '#150f07', line: '#3b2c12' },
  Lunar: { accent: '#dbeafe', accent2: '#93c5fd', bg: '#090d16', panel: '#141b29', panel2: '#0e141f', line: '#26344a' },
  Plasma: { accent: '#e879f9', accent2: '#8b5cf6', bg: '#130718', panel: '#20102a', panel2: '#170b20', line: '#3e1f4f' },
  Midnight: { accent: '#818cf8', accent2: '#22d3ee', bg: '#050711', panel: '#0d1221', panel2: '#090e19', line: '#1e2942' },
  Storm: { accent: '#93c5fd', accent2: '#64748b', bg: '#080e16', panel: '#111a25', panel2: '#0d151e', line: '#263646' },
  Monsoon: { accent: '#2dd4bf', accent2: '#38bdf8', bg: '#06110f', panel: '#0d1b19', panel2: '#091411', line: '#1e3a35' },
  Velvet: { accent: '#d946ef', accent2: '#f472b6', bg: '#160811', panel: '#21101b', panel2: '#170b13', line: '#3e1d32' },
  Cobalt: { accent: '#2563eb', accent2: '#60a5fa', bg: '#060b16', panel: '#0d1627', panel2: '#09111f', line: '#1e3354' },
  Magenta: { accent: '#f472b6', accent2: '#e879f9', bg: '#140812', panel: '#201020', panel2: '#170c18', line: '#3b1d3a' },
  Icefire: { accent: '#67e8f9', accent2: '#f97316', bg: '#0a0e14', panel: '#14191f', panel2: '#0f1318', line: '#2d3942' },
  Quartz: { accent: '#e2e8f0', accent2: '#94a3b8', bg: '#0a0d12', panel: '#171c23', panel2: '#10151b', line: '#29333f' },
  Halo: { accent: '#f8fafc', accent2: '#fde68a', bg: '#0b0d12', panel: '#181b23', panel2: '#11141a', line: '#2d313b' },
  Nova: { accent: '#60a5fa', accent2: '#e879f9', bg: '#090817', panel: '#141127', panel2: '#0e0b1e', line: '#2c2450' },
  Circuit: { accent: '#84cc16', accent2: '#22c55e', bg: '#07100a', panel: '#0d1810', panel2: '#091309', line: '#20351f' },
  Horizon: { accent: '#2dd4bf', accent2: '#fb923c', bg: '#08100f', panel: '#111b18', panel2: '#0c1512', line: '#243b32' },
  Dusk: { accent: '#c084fc', accent2: '#fb923c', bg: '#110b13', panel: '#1a1320', panel2: '#120c17', line: '#352542' },
  Dawn: { accent: '#fde68a', accent2: '#fb7185', bg: '#170f0b', panel: '#21170f', panel2: '#160f0a', line: '#3d2a1a' },
  Mirage: { accent: '#67e8f9', accent2: '#c084fc', bg: '#071016', panel: '#101925', panel2: '#0b121c', line: '#25364a' },
  Eclipse: { accent: '#a78bfa', accent2: '#475569', bg: '#05060a', panel: '#0e1017', panel2: '#090b10', line: '#1f2430' },
  Flux: { accent: '#22d3ee', accent2: '#a3e635', bg: '#06100f', panel: '#0f1b1b', panel2: '#0a1414', line: '#203b3b' },
  Lagoon: { accent: '#2dd4bf', accent2: '#06b6d4', bg: '#06100f', panel: '#0d1a19', panel2: '#091311', line: '#1d3735' },
  Boreal: { accent: '#22c55e', accent2: '#84cc16', bg: '#07100a', panel: '#0d1a10', panel2: '#09130c', line: '#1d3822' },
  Onyx: { accent: '#94a3b8', accent2: '#475569', bg: '#050608', panel: '#0e1116', panel2: '#080a0d', line: '#1e252d' },
  Pearl: { accent: '#f8fafc', accent2: '#cbd5e1', bg: '#0c0f14', panel: '#171c23', panel2: '#11151c', line: '#2a333e' },
  Copper: { accent: '#fb923c', accent2: '#d97706', bg: '#120b07', panel: '#1c110b', panel2: '#130c08', line: '#372015' },
  Indigo: { accent: '#6366f1', accent2: '#3b82f6', bg: '#090a17', panel: '#11152a', panel2: '#0c1020', line: '#272e4b' },
  Jade: { accent: '#10b981', accent2: '#22c55e', bg: '#06100d', panel: '#0d1914', panel2: '#09140f', line: '#1e382b' },
  Scarlet: { accent: '#ef4444', accent2: '#fb7185', bg: '#150709', panel: '#1f0e12', panel2: '#150a0d', line: '#3a1b22' },
  Polar: { accent: '#bae6fd', accent2: '#93c5fd', bg: '#071018', panel: '#0f1a24', panel2: '#0a141e', line: '#223b4d' },
  Astral: { accent: '#a78bfa', accent2: '#67e8f9', bg: '#0b0716', panel: '#141024', panel2: '#0d0b1a', line: '#2c2143' },
};
const PAID_FONT_STACKS = [
  'Bahnschrift, "Open Sans", sans-serif',
  '"Segoe UI Semibold", "Open Sans", sans-serif',
  'Trebuchet MS, "Open Sans", sans-serif',
  'Consolas, "Open Sans", monospace',
  'Arial Narrow, "Open Sans", sans-serif',
  'Cambria, "Open Sans", serif',
  'Franklin Gothic Medium, "Open Sans", sans-serif',
  'Candara, "Open Sans", sans-serif',
];
const THEME_FONTS: Record<string,string> = {
  Aurora: '"Trebuchet MS", "Open Sans", sans-serif', Sunset: 'Georgia, "Open Sans", serif', Cyber: 'Consolas, "Open Sans", monospace', Violet: '"Segoe UI", "Open Sans", sans-serif', Nebula: '"Trebuchet MS", "Open Sans", sans-serif', Velvet: 'Georgia, "Open Sans", serif', Copper: '"Trebuchet MS", "Open Sans", sans-serif', Quartz: '"Segoe UI Light", "Open Sans", sans-serif', Pearl: '"Segoe UI Light", "Open Sans", sans-serif', Onyx: 'Bahnschrift, "Open Sans", sans-serif', Circuit: 'Consolas, "Open Sans", monospace', Prism: '"Trebuchet MS", "Open Sans", sans-serif'
};
function themeForIndex(index: number, name?: string) {
  const baseName = name ?? BASE_THEME_NAMES[(Math.max(1,index)-1) % BASE_THEME_NAMES.length];
  const palette = THEME_PALETTES[baseName] ?? THEME_PALETTES.Obsidian;
  const accentShift = Math.floor(Math.max(0,index) / 8) % 2;
  return { ...palette, accent2: accentShift ? palette.accent : palette.accent2, text: baseName === 'Glass' ? '#0f172a' : '#e8eef7', font: THEME_FONTS[baseName] ?? '"Open Sans", "Segoe UI", sans-serif' };
}

const THEME_CATALOG: Reward[] = Array.from({ length: 100 }, (_, index) => {
  const n = index + 1;
  const id = index < 8 ? FREE_THEME_IDS[index] : `theme:theme-${String(n).padStart(3, '0')}`;
  const base = BASE_THEME_NAMES[index % BASE_THEME_NAMES.length];
  const mode = THEME_MODES[Math.floor(index / BASE_THEME_NAMES.length) % THEME_MODES.length];
  const style = themeForIndex(index + 1, base);
  const price = index < 8 ? 0 : index >= 92 ? 3000 + (index - 92) * 260 : 180 + Math.floor((index - 8) / 8) * 170 + ((index - 8) % 8) * 35;
  return { id, name: index < 8 ? base : `${base} ${mode}`, description: index < 8 ? 'Free core theme with a polished Digital Balance finish.' : `A collectible ${base.toLowerCase()} skin with ${mode.toLowerCase()} lighting and contrast.`, price, kind: 'theme', accent: style.accent };
});

const EXTRA_REWARDS: Reward[] = [
  { id: 'frame:neon', name: 'Neon Frame', description: 'Animated glow around your identity chip.', price: 300, kind: 'frame', accent: '#22d3ee' },
  { id: 'frame:gold', name: 'Gold Frame', description: 'Premium animated-gold profile edge.', price: 450, kind: 'frame', accent: '#facc15' },
  { id: 'frame:prism', name: 'Prism Frame', description: 'Color-shifting frame with a soft spectrum glow.', price: 900, kind: 'frame', accent: '#c084fc' },
  { id: 'effect:orbit', name: 'Orbit Focus', description: 'Orbit rings around the Focus timer.', price: 400, kind: 'effect', accent: '#818cf8' },
  { id: 'effect:pulse', name: 'Pulse Focus', description: 'Rhythmic focus glow while a session runs.', price: 350, kind: 'effect', accent: '#34d399' },
  { id: 'effect:matrix', name: 'Matrix Focus', description: 'Sharper scanline motion around the timer.', price: 750, kind: 'effect', accent: '#a3e635' },
  { id: 'effect:nebula', name: 'Nebula Drift', description: 'Premium multi-orb ambience that slowly shifts through the session.', price: 1200, kind: 'effect', accent: '#8b5cf6' },
  { id: 'effect:prism', name: 'Prism Bloom', description: 'A soft chromatic bloom that changes palette as focus progresses.', price: 1500, kind: 'effect', accent: '#22d3ee' },
  { id: 'effect:golden', name: 'Golden Hour', description: 'Warm breathing light that intensifies near the final stretch.', price: 1800, kind: 'effect', accent: '#f59e0b' },
];
const rewards: Reward[] = [...THEME_CATALOG, ...EXTRA_REWARDS];

const iconPaths: Record<string, string> = {
  grid: 'M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z',
  focus: 'M12 3v3M21 12h-3M12 21v-3M3 12h3M16.2 7.8a6 6 0 1 0 0 8.4',
  insights: 'M5 19V9M12 19V5M19 19V3M3 19h18',
  trophy: 'M8 4h8v4a4 4 0 0 1-8 0V4ZM8 6H5a3 3 0 0 0 3 4M16 6h3a3 3 0 0 1-3 4M12 12v4M8 20h8M9 16h6',
  control: 'M4 7h10M18 7h2M4 12h2M10 12h10M4 17h10M18 17h2M14 5v4M8 10v4M14 15v4',
  shop: 'M4 8h16l-1 12H5L4 8ZM8 8a4 4 0 0 1 8 0M8 12h.01M16 12h.01',
  settings: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM4.9 4.9l2.1 2.1M17 17l2.1 2.1M4 12H1M23 12h-3M4.9 19.1 7 17M17 7l2.1-2.1M12 4V1M12 23v-3',
  menu: 'M4 7h16M4 12h16M4 17h16',
  spark: 'M12 2l1.8 6.2L20 10l-6.2 1.8L12 18l-1.8-6.2L4 10l6.2-1.8L12 2z',
};
function Icon({ name, size = 17 }: { name: string; size?: number }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={iconPaths[name] ?? iconPaths.grid} /></svg>;
}
function BrandMark({ big = false }: { big?: boolean }) {
  return <div className={`brand-mark ${big ? 'big' : ''}`} aria-label="Digital Balance"><svg viewBox="0 0 48 48" aria-hidden="true"><defs><linearGradient id="db-g" x1="6" y1="6" x2="42" y2="42"><stop offset="0" stopColor="var(--accent)"/><stop offset="1" stopColor="var(--accent2)"/></linearGradient></defs><path d="M8 8h19c7 0 13 6 13 13s-6 13-13 13H8V8Z" fill="url(#db-g)" opacity=".95"/><path d="M17 14h8.5a7 7 0 0 1 0 14H17V14Z" fill="#081018" opacity=".95"/><path d="M17 18h8.2a3 3 0 1 1 0 6H17v-6Z" fill="#e8eef7" opacity=".92"/><path d="M31 14v20" stroke="url(#db-g)" strokeWidth="3" strokeLinecap="round"/><circle cx="35" cy="36" r="3" fill="url(#db-g)"/></svg></div>;
}

type SmartSelectProps = { value: string; onChange: (value: string) => void; options: { value: string; label: string }[]; placeholder?: string; ariaLabel?: string };
function SmartSelect({ value, onChange, options, placeholder = 'Choose…', ariaLabel }: SmartSelectProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const onDoc = (event: MouseEvent) => { if (!ref.current?.contains(event.target as Node)) setOpen(false); };
    document.addEventListener('mousedown', onDoc); return () => document.removeEventListener('mousedown', onDoc);
  }, []);
  const selected = options.find(option => option.value === value);
  return <div className={`smart-select ${open ? 'open' : ''}`} ref={ref}>
    <button type="button" className="smart-select-trigger" aria-label={ariaLabel} aria-expanded={open} onClick={() => setOpen(v => !v)}><span>{selected?.label ?? placeholder}</span><span className="smart-select-chevron">⌄</span></button>
    {open && <div className="smart-select-menu">{options.map(option => <button type="button" key={option.value} className={`smart-select-option ${option.value === value ? 'selected' : ''}`} onClick={() => { onChange(option.value); setOpen(false); }}>{option.label}</button>)}</div>}
  </div>;
}
const time = (seconds: number) => { const total = Math.max(0, Math.round(seconds)); const h = Math.floor(total / 3600); const m = Math.floor((total % 3600) / 60); const s = total % 60; if (h) return `${h}h ${m}m`; if (m) return `${m}m ${s}s`; return `${s}s`; };
const minutes = (seconds: number) => Math.round(seconds / 60);
const randomName = () => AESTHETIC_NAMES[Math.floor(Math.random() * AESTHETIC_NAMES.length)];
const themeIndexFromId = (id: string) => { const m = id.match(/theme-(\d+)/); return m ? Number(m[1]) : Math.max(1, FREE_THEME_IDS.indexOf(id) + 1); };
const themeById = (id: string) => THEME_CATALOG.find((theme) => theme.id === id) ?? THEME_CATALOG[0];

function focusStreak(data: TrackerData) { const dates = new Set(data.focus_sessions.map((s) => new Date(s.completed_at).toDateString())); let count = 0; const cursor = new Date(); cursor.setHours(12,0,0,0); while (dates.has(cursor.toDateString())) { count += 1; cursor.setDate(cursor.getDate()-1); } return count; }
function scoreFor(data: TrackerData) { const day = new Date().toDateString(); const sessions = data.sessions.filter((s) => new Date(s.started_at).toDateString() === day); const productive = sessions.filter((s) => s.category === 'Productive').reduce((a,s) => a+s.duration_secs,0); const distracting = sessions.filter((s) => s.category === 'Distracting').reduce((a,s) => a+s.duration_secs,0); const focus = data.focus_sessions.filter((s) => new Date(s.completed_at).toDateString() === day).reduce((a,s) => a+s.minutes,0); const limit = Math.max(1,data.settings.dailyDistractionLimitMinutes); const target = Math.max(1,data.settings.dailyFocusTargetMinutes); const balance = distracting/60 <= limit ? 100 : Math.max(25,100-((distracting/60-limit)*4)); const focusScore = Math.min(100,(focus/target)*100); const productiveRatio = productive+distracting>0 ? (productive/(productive+distracting))*100 : 70; return Math.round(balance*.45+focusScore*.35+productiveRatio*.2); }

function isProtectedCandidate(exe: string) { return exe && !PROTECTED_EXES.has(exe.toLowerCase()); }

export default function App() {
  const [session, setSession] = useState<Session | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [authReady, setAuthReady] = useState(!supabaseConfigured);
  const [view, setView] = useState<View>('Overview');
  const [data, setData] = useState<TrackerData | null>(null);
  const dataRef = useRef<TrackerData | null>(null);
  const [leaderboard, setLeaderboard] = useState<LeaderboardRow[]>([]);
  const [notice, setNotice] = useState('');
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [closePrompt, setClosePrompt] = useState(false);
  const [focusMinutes, setFocusMinutes] = useState(25);
  const [focusRemaining, setFocusRemaining] = useState(25 * 60);
  const [focusRunning, setFocusRunning] = useState(false);
  const [focusCompleted, setFocusCompleted] = useState(false);
  const [focusApps, setFocusApps] = useState<string[]>([]);
  const [shopFilter, setShopFilter] = useState<'theme' | 'frame' | 'effect'>('theme');
  const [themeSearch, setThemeSearch] = useState('');
  const [themePreview, setThemePreview] = useState<string | null>(null);
  const [quote, setQuote] = useState<string | null>(null);
  const allowCloseRef = useRef(false);
  const nextQuoteRef = useRef(Date.now() + 8 * 60 * 1000);
  const focusRestoreSizeRef = useRef<{ width: number; height: number } | null>(null);
  const focusRestoreFullscreenRef = useRef(false);
  const [focusFullscreen, setFocusFullscreen] = useState(false);
  const [update, setUpdate] = useState<Update | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);

  useEffect(() => { dataRef.current = data; }, [data]);

  useEffect(() => {
    const themeId = themePreview ?? data?.settings.theme ?? 'theme:obsidian';
    const index = themeIndexFromId(themeId);
    const named = themeById(themeId);
    const baseName = named.name.replace(/ (Core|Flux)$/,'');
    const styleTheme = themeForIndex(index, baseName);
    const root = document.documentElement;
    root.dataset.theme = themeId.replace('theme:', '');
    root.dataset.frame = data?.settings.equippedFrame ?? 'frame:classic';
    root.dataset.effect = data?.settings.equippedEffect ?? 'effect:clean';
    root.dataset.mode = (data?.settings.mode ?? 'Smart').replace(' ', '-');
    root.style.setProperty('--accent', named?.accent ?? styleTheme.accent);
    root.style.setProperty('--accent2', styleTheme.accent2);
    root.style.setProperty('--bg', styleTheme.bg);
    root.style.setProperty('--panel', styleTheme.panel);
    root.style.setProperty('--panel2', styleTheme.panel2);
    root.style.setProperty('--line', styleTheme.line);
    root.style.setProperty('--text', styleTheme.text);
    root.style.setProperty('--app-font', styleTheme.font);
    const freeCustom = themeId === 'theme:aurora' || themeId === 'theme:sunset';
    const paidFont = named.price > 0 ? PAID_FONT_STACKS[index % PAID_FONT_STACKS.length] : '"Open Sans", "Segoe UI", sans-serif';
    const customFontEnabled = freeCustom || named.price > 0;
    root.style.setProperty('--app-font', customFontEnabled ? (THEME_FONTS[baseName] ?? paidFont) : '"Open Sans", "Segoe UI", sans-serif');
    root.style.setProperty('--display-font', customFontEnabled ? (THEME_FONTS[baseName] ?? paidFont) : '"Open Sans", "Segoe UI", sans-serif');
    const FONT_SCALE: Record<string, number> = { Aurora: .96, Sunset: .91, Cyber: .92, Violet: 1, Nebula: .96, Velvet: .91, Copper: .96, Quartz: .94, Pearl: .94, Onyx: .97, Circuit: .92, Prism: .96 };
    root.style.setProperty('--font-scale', customFontEnabled ? String(FONT_SCALE[baseName] ?? 1) : '1');
  }, [data?.settings.theme, data?.settings.equippedFrame, data?.settings.equippedEffect, themePreview]);

  useEffect(() => { if (data?.settings.deepFocusBlockedApps) setFocusApps(data.settings.deepFocusBlockedApps); }, [data?.settings.deepFocusBlockedApps]);

  useEffect(() => {
    let alive = true;
    void check().then((next) => {
      if (alive && next) setUpdate(next);
    }).catch(() => {
      // Update checks are intentionally silent when offline or when the updater
      // has not been configured for the development build.
    });
    return () => { alive = false; };
  }, []);

  async function installAvailableUpdate() {
    if (!update) return;
    setUpdateBusy(true);
    try {
      await update.downloadAndInstall();
    } catch (error) {
      setUpdateBusy(false);
      setNotice(error instanceof Error ? `Update failed: ${error.message}` : 'Update could not be installed.');
    }
  }

  async function dismissAvailableUpdate() {
    if (!update) return;
    try { await update.close(); } catch { /* ignore */ }
    setUpdate(null);
  }

  useEffect(() => {
    let dispose: () => void = () => undefined;
    void setupDeepLinkAuth().then((cleanup) => { dispose = cleanup; });
    if (!supabase) { setAuthReady(true); return () => dispose(); }
    const current = supabase.auth.getSession().then(({ data: result }) => { setSession(result.session); setUser(result.session?.user ?? null); setAuthReady(true); if (result.session?.user) void ensureProfile(result.session.user).catch(() => undefined); });
    const { data: listener } = supabase.auth.onAuthStateChange((_event, next) => { setSession(next); setUser(next?.user ?? null); setAuthReady(true); if (next?.user) void ensureProfile(next.user).catch(() => undefined); });
    return () => { dispose(); void current; listener.subscription.unsubscribe(); };
  }, []);

  useEffect(() => {
    let alive = true;
    const tick = async () => { if (document.hidden) return; try { const next = await getTrackerData(); if (alive) setData(next); } catch (error) { console.error(error); } };
    void tick();
    const id = window.setInterval(tick, 5000);
    return () => { alive = false; window.clearInterval(id); };
  }, []);

  useEffect(() => {
    const unlisten = listen<{ exe: string; app: string; reason: string; usedMinutes: number; limitMinutes: number }>('app-limit-hit', (event) => {
      setNotice(event.payload.limitMinutes > 0 ? `${event.payload.app} reached its ${event.payload.limitMinutes} min protection. ${event.payload.reason}.` : `${event.payload.app} was closed by Digital Balance. ${event.payload.reason}.`);
      setQuote(QUOTES[Math.floor(Math.random() * QUOTES.length)]);
      nextQuoteRef.current = Date.now() + 12 * 60 * 1000;
      void getTrackerData().then(setData).catch(() => undefined);
    });
    return () => { void unlisten.then((off) => off()); };
  }, []);

  useEffect(() => {
    if (!data?.settings.motivationalNudges || !data.settings.notificationsEnabled) return;
    const id = window.setInterval(() => {
      if (dataRef.current?.current_category === 'Distracting' && (dataRef.current.current_session_secs ?? 0) >= 10 * 60 && Date.now() >= nextQuoteRef.current) {
        setQuote(QUOTES[Math.floor(Math.random() * QUOTES.length)]);
        nextQuoteRef.current = Date.now() + (10 + Math.floor(Math.random() * 8)) * 60 * 1000;
      }
    }, 60 * 1000);
    return () => window.clearInterval(id);
  }, [data?.settings.motivationalNudges, data?.settings.notificationsEnabled]);

  useEffect(() => {
    if (!focusRunning) return;
    if (focusRemaining <= 0) { void finishFocus(); return; }
    const id = window.setInterval(() => setFocusRemaining((value) => Math.max(0, value - 1)), 1000);
    return () => window.clearInterval(id);
  }, [focusRunning, focusRemaining]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); setPaletteOpen((value) => !value); }
      if (event.key === 'Escape') { setPaletteOpen(false); setQuote(null); setClosePrompt(false); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);


  useEffect(() => {
    const win = getCurrentWindow();
    let off: (() => void) | undefined;
    void win.onCloseRequested((event) => {
      if (allowCloseRef.current) return;
      if (dataRef.current?.settings.closeToTray ?? true) {
        event.preventDefault();
        setClosePrompt(true);
      }
    }).then((cleanup) => { off = cleanup; });
    return () => { off?.(); };
  }, []);

  const today = useMemo(() => {
    if (!data) return { tracked: 0, productive: 0, distracting: 0, focus: 0 };
    const key = new Date().toDateString(); const sessions = data.sessions.filter((s) => new Date(s.started_at).toDateString() === key);
    return { tracked: sessions.reduce((a,s)=>a+s.duration_secs,0)+data.current_session_secs, productive: sessions.filter((s)=>s.category==='Productive').reduce((a,s)=>a+s.duration_secs,0), distracting: sessions.filter((s)=>s.category==='Distracting').reduce((a,s)=>a+s.duration_secs,0), focus: data.focus_sessions.filter((s)=>new Date(s.completed_at).toDateString()===key).reduce((a,s)=>a+s.minutes,0) };
  }, [data]);

  const topApps = useMemo(() => {
    if (!data) return [];
    const map = new Map<string,{name:string;exe:string;category:Category;secs:number}>();
    for (const s of data.sessions) { if (!isProtectedCandidate(s.exe)) continue; const row = map.get(s.exe) ?? { name:s.app_name, exe:s.exe, category:s.category, secs:0 }; row.secs += s.duration_secs; row.category = s.category; map.set(s.exe,row); }
    if (data.current_exe && isProtectedCandidate(data.current_exe)) { const row=map.get(data.current_exe) ?? {name:data.current_app ?? data.current_exe,exe:data.current_exe,category:data.current_category ?? 'Neutral',secs:0}; row.secs += data.current_session_secs; map.set(data.current_exe,row); }
    return [...map.values()].sort((a,b)=>b.secs-a.secs).slice(0,16);
  }, [data]);

  const appOptions = topApps.filter(app => isProtectedCandidate(app.exe));
  const weekly = useMemo(() => !data ? [] : Array.from({length:7},(_,index)=>{ const date=new Date(); date.setHours(12,0,0,0); date.setDate(date.getDate()-(6-index)); const key=date.toDateString(); const rows=data.sessions.filter(s=>new Date(s.started_at).toDateString()===key); const productive=rows.filter(s=>s.category==='Productive').reduce((a,s)=>a+s.duration_secs,0); const distracting=rows.filter(s=>s.category==='Distracting').reduce((a,s)=>a+s.duration_secs,0); return {label:date.toLocaleDateString(undefined,{weekday:'short'}),productive,distracting,total:productive+distracting}; }), [data]);
  const streak = data ? focusStreak(data) : 0;
  const score = data ? scoreFor(data) : 0;
  const displayName = data?.settings.displayName || user?.user_metadata?.name || user?.user_metadata?.full_name || user?.email?.split('@')[0] || 'Digital Balance';

  async function refreshLeaderboard() { if (!supabase) return; try { setLeaderboard(await fetchLeaderboard()); } catch (error) { console.error(error); setNotice('Could not load the global leaderboard.'); } }
  useEffect(() => { if (view === 'Leaderboard' && user) void refreshLeaderboard(); }, [view, user]);

  async function enterFocusWindow() {
    const win = getCurrentWindow();
    try {
      focusRestoreFullscreenRef.current = await win.isFullscreen();
      if (focusRestoreFullscreenRef.current) await win.setFullscreen(false);
      const size = await win.innerSize();
      const scale = await win.scaleFactor();
      const logical = size.toLogical(scale);
      focusRestoreSizeRef.current = { width: logical.width, height: logical.height };
      await win.setSize(new LogicalSize(760, 540));
      await win.center();
      setFocusFullscreen(false);
    } catch (error) { console.warn('Focus mini-player resize skipped', error); }
  }

  async function restoreFocusWindow() {
    const win = getCurrentWindow();
    try {
      if (await win.isFullscreen()) await win.setFullscreen(false);
      if (focusRestoreSizeRef.current) {
        await win.setSize(new LogicalSize(focusRestoreSizeRef.current.width, focusRestoreSizeRef.current.height));
        await win.center();
      }
      if (focusRestoreFullscreenRef.current) await win.setFullscreen(true);
    } catch (error) { console.warn('Focus window restore skipped', error); }
    focusRestoreSizeRef.current = null;
    focusRestoreFullscreenRef.current = false;
    setFocusFullscreen(false);
  }

  async function toggleFocusFullscreen() {
    const win = getCurrentWindow();
    try {
      const next = !(await win.isFullscreen());
      await win.setFullscreen(next);
      setFocusFullscreen(next);
    } catch (error) { console.warn('Fullscreen toggle failed', error); }
  }

  async function finishFocus() {
    if (!focusRunning && focusRemaining > 0) return;
    setFocusRunning(false);
    await setFocusProtection(false, focusApps, data?.settings.mode ?? 'Smart').catch(()=>undefined);
    try {
      const result=await completeFocusSession(focusMinutes,data?.settings.mode ?? 'Smart');
      if(user&&supabase){ const cloud=await awardFocusPoints(focusMinutes); setNotice(`Focus complete — +${result.coins} Balance Coins and ${cloud.points} leaderboard points secured.`); }
      else setNotice(`Focus complete — +${result.coins} Balance Coins.`);
      setFocusCompleted(true); setFocusRemaining(focusMinutes*60); setData(result.data);
      window.setTimeout(()=>{ setFocusCompleted(false); void restoreFocusWindow(); },3200);
    } catch(error){ console.error(error); setNotice('Focus finished, but saving the rewards failed.'); await restoreFocusWindow(); }
  }
  async function startFocus() {
    if(!data)return;
    await enterFocusWindow();
    setView('Focus');
    setFocusRemaining(focusRemaining<=0?focusMinutes*60:focusRemaining);
    setFocusRunning(true);
    await setFocusProtection(true,focusApps,data.settings.mode).then(setData).catch(()=>undefined);
  }
  async function pauseFocus() { setFocusRunning(false); await setFocusProtection(false,focusApps,data?.settings.mode ?? 'Smart').catch(()=>undefined); }
  async function changeMode(mode: AppMode) { if(!data)return; setData(await saveSettings({...data.settings,mode})); if(focusRunning){ await setFocusProtection(true,focusApps,mode).then(setData).catch(()=>undefined); } }
  async function togglePause() { if(!data)return; setData(await setTrackingPaused(!data.tracking_paused)); }
  async function saveLimit(exe:string, limit:AppLimit) { if(!data)return; try{setData(await saveAppLimit(exe,limit));setNotice(`${exe} protection saved.`);}catch(error){console.error(error);setNotice(error instanceof Error?error.message:'Could not save that app protection.');} }
  async function removeLimit(exe:string){ if(!data)return; setData(await removeAppLimit(exe)); setNotice(`${exe} protection removed.`); }
  async function buyReward(reward:Reward){ if(!data)return; try{setData(await purchaseReward(reward.id));setNotice(`${reward.name} unlocked.`);}catch(error){setNotice(error instanceof Error?error.message:'You need more Balance Coins.');} }
  async function equip(reward:Reward){ if(!data)return; try{setData(await equipReward(reward.id));setNotice(`${reward.name} equipped.`);}catch(error){setNotice(error instanceof Error?error.message:'Could not equip that reward.');} }
  async function applyTheme(themeId:string){ if(!data)return; setThemePreview(null); setData(await saveSettings({...data.settings,theme:themeId})); setNotice(`${themeById(themeId).name} equipped.`); }
  async function saveDisplayName(name:string){ if(!data)return; const clean=name.trim().slice(0,28); if(!clean){setNotice('Choose a display name first.');return;} setData(await saveSettings({...data.settings,displayName:clean})); if(user&&supabase){ try{await updateProfileDisplayName(clean);}catch(error){console.error(error);setNotice('Saved locally, but cloud profile update failed.');return;} } setNotice(`Identity set to ${clean}.`); }

  if(!authReady) return <Splash/>;
  if(!supabaseConfigured) return <CloudSetupScreen issue={supabaseConfigIssue ?? 'Cloud authentication is not configured.'}/>;
  if(!session) return <LoginScreen onLogin={signInWithGoogle} notice={notice}/>;
  if(!data) return <Splash/>;

  const appName=data.current_app ?? 'No foreground app';
  const currentLimit=data.current_exe?data.settings.appLimits[data.current_exe]:undefined;
  const currentRemaining=currentLimit?Math.max(0,currentLimit.dailyMinutes-minutes(todayForExe(data,data.current_exe!))):null;
  const go=(next:View)=>{setPaletteOpen(false);setView(next);};
  const rootClass=`app-shell ${sidebarCollapsed?'sidebar-collapsed':''} ${(focusRunning || focusCompleted)?'focus-active':''}`;

  return <div className={rootClass} data-mode={data.settings.mode.replace(' ', '-')}>
    <aside className="sidebar">
      <div className="brand-row"><button className="sidebar-toggle" onClick={()=>setSidebarCollapsed(v=>!v)} title={sidebarCollapsed?'Show sidebar':'Hide sidebar'}><Icon name="menu" size={17}/></button><div className="brand"><BrandMark />{!sidebarCollapsed&&<div><strong>Digital Balance</strong><span>Own your screen time.</span></div>}</div></div>
      {!sidebarCollapsed&&<div className="sidebar-meta"><span className="live-dot"/> {data.tracking_paused?'Tracking paused':'System monitor active'}<small>Event-driven + low-power monitor</small></div>}
      <nav className="nav-list">{([['Overview','grid'],['Focus','focus'],['Insights','insights'],['Control','control'],['Rewards','shop'],['Leaderboard','trophy'],['Settings','settings']] as [View,string][]).map(([id,icon])=><button key={id} className={`nav-item ${view===id?'active':''}`} onClick={()=>go(id)} title={id}><Icon name={icon}/>{!sidebarCollapsed&&<span>{id}</span>}</button>)}</nav>
      <div className="sidebar-bottom"><div className="coin-mini"><span>{sidebarCollapsed?'Coins':'Balance Coins'}</span><strong>{data.settings.coins.toLocaleString()}</strong></div><button className="pause-btn" onClick={()=>void togglePause()}>{sidebarCollapsed?(data.tracking_paused?'▶':'Ⅱ'):(data.tracking_paused?'Resume tracking':'Pause tracking')}</button></div>
    </aside>

    <main className="main-pane">
      <header className="topbar"><div><div className="eyebrow">{new Date().toLocaleDateString(undefined,{weekday:'long',month:'long',day:'numeric'})}</div><h1>{view}</h1></div><div className="top-actions"><button className="command-btn" onClick={()=>setPaletteOpen(true)}><span>Command</span><kbd>Ctrl K</kbd></button><div className="mode-switch">{modes.map(mode=><button key={mode} className={data.settings.mode===mode?'selected':''} onClick={()=>void changeMode(mode)}>{mode}</button>)}</div><div className="profile-chip"><div className="avatar">{displayName.slice(0,1).toUpperCase()}</div><strong>{displayName}</strong><button className="signout" onClick={()=>void supabase?.auth.signOut()}>Sign out</button></div></div></header>
      {notice&&<button className="notice" onClick={()=>setNotice('')}><span>{notice}</span><span>×</span></button>}
      {update&&<div className="update-banner"><div><span className="section-label">UPDATE READY</span><strong>Digital Balance {update.version} is available.</strong><small>Your current version can update without downloading another installer.</small></div><div className="update-actions"><button className="ghost-btn small" onClick={()=>void dismissAvailableUpdate()} disabled={updateBusy}>Later</button><button className="primary-btn small" onClick={()=>void installAvailableUpdate()} disabled={updateBusy}>{updateBusy?'Installing…':'Update now'}</button></div></div>}
      {quote&&<div className="motivation-toast"><div><span className="section-label">LOCK IN</span><strong>{quote}</strong><small>Digital Balance noticed a distraction pattern.</small></div><div className="motivation-actions"><button className="ghost-btn small" onClick={()=>setQuote(null)}>Dismiss</button><button className="primary-btn small" onClick={()=>{setQuote(null);setView('Focus');void startFocus();}}>Start Focus</button></div></div>}
      {view==='Overview'&&<Overview data={data} score={score} today={today} topApps={topApps} streak={streak} onStartFocus={()=>{setView('Focus');void startFocus();}} onGo={setView} currentRemaining={currentRemaining} appName={appName}/>} 
      {view==='Focus'&&<FocusPage data={data} minutes={focusMinutes} remaining={focusRemaining} running={focusRunning} completed={focusCompleted} focusApps={focusApps} setFocusApps={setFocusApps} knownApps={appOptions} onDuration={v=>{setFocusMinutes(v);setFocusRemaining(v*60)}} onStart={startFocus} onPause={pauseFocus} onReset={()=>{setFocusRunning(false);setFocusRemaining(focusMinutes*60);void setFocusProtection(false,focusApps,data.settings.mode).then(()=>restoreFocusWindow()).catch(()=>restoreFocusWindow())}} onToggleFullscreen={toggleFocusFullscreen} fullscreen={focusFullscreen}/>}
      {view==='Insights'&&<Insights data={data} weekly={weekly} topApps={topApps}/>} 
      {view==='Control'&&<ControlPage data={data} topApps={appOptions} onSaveLimit={saveLimit} onRemoveLimit={removeLimit} focusApps={focusApps} setFocusApps={setFocusApps}/>} 
      {view==='Rewards'&&<RewardsPage data={data} rewards={rewards} filter={shopFilter} setFilter={setShopFilter} search={themeSearch} setSearch={setThemeSearch} preview={themePreview} onPreview={setThemePreview} onBuy={buyReward} onEquip={equip} onApplyTheme={applyTheme} onResetPreview={()=>setThemePreview(null)}/>} 
      {view==='Leaderboard'&&<Leaderboard data={leaderboard} userId={user?.id ?? ''} refresh={refreshLeaderboard}/>} 
      {view==='Settings'&&<SettingsPage data={data} setData={setData} user={user} onSaveDisplayName={saveDisplayName}/>} 
    </main>

    {paletteOpen&&<CommandPalette onClose={()=>setPaletteOpen(false)} onGo={go} onStartFocus={()=>{setPaletteOpen(false);setView('Focus');void startFocus();}}/>}
    {closePrompt&&<ClosePrompt onBackground={async()=>{setClosePrompt(false);try{await hideToTray();}catch(error){setNotice(error instanceof Error?error.message:'Could not move Digital Balance to the system tray.');}}} onQuit={async()=>{setClosePrompt(false);try{await quitApp();}catch(error){setNotice(error instanceof Error?error.message:'Could not close Digital Balance.');}}}/>}

  </div>;
}

function todayForExe(data:TrackerData,exe:string){const key=new Date().toDateString();let total=data.sessions.filter(s=>s.exe===exe&&new Date(s.started_at).toDateString()===key).reduce((a,s)=>a+s.duration_secs,0);if(data.current_exe===exe)total+=data.current_session_secs;return total;}
function Splash(){return <div className="splash"><div className="loader-orbit"/><strong>Starting Digital Balance</strong><span>Bringing the tracker online…</span></div>;}
function CloudSetupScreen({issue}:{issue:string}){return <div className="login-screen"><div className="login-card"><BrandMark big /><div className="eyebrow">Cloud setup needed</div><h1>One last connection.</h1><p>Digital Balance requires its Supabase connection before anyone can use the account and global leaderboard.</p><div className="setup-error"><strong>{issue}</strong><code>Create <b>.env</b> beside package.json with VITE_SUPABASE_URL and VITE_SUPABASE_PUBLISHABLE_KEY.</code></div><button className="primary-btn wide" onClick={()=>window.location.reload()}>Retry connection</button></div></div>;}
function LoginScreen({onLogin,notice}:{onLogin:()=>Promise<void>;notice:string}){return <div className="login-screen"><div className="login-card"><BrandMark big /><div className="eyebrow">Digital Balance</div><h1>A smarter relationship with your screen.</h1><p>Track Windows usage, set real limits, earn rewards, run Focus, and compare progress without sending detailed activity to the cloud.</p><button className="google-btn" onClick={()=>void onLogin()}><span>G</span> Continue with Google</button><div className="privacy-note">Detailed activity remains local. Only the minimum leaderboard data is synchronized.</div>{notice&&<div className="login-error">{notice}</div>}</div></div>;}

function Overview({data,score,today,topApps,streak,onStartFocus,onGo,currentRemaining,appName}:{data:TrackerData;score:number;today:{tracked:number;productive:number;distracting:number;focus:number};topApps:{name:string;exe:string;category:Category;secs:number}[];streak:number;onStartFocus:()=>void;onGo:(view:View)=>void;currentRemaining:number|null;appName:string}){const progress=Math.min(100,(today.focus/Math.max(1,data.settings.dailyFocusTargetMinutes))*100);return <div className="content-grid"><section className="hero-grid"><div className="hero-card hero-main"><span className="section-label">DIGITAL BALANCE SCORE</span><div className="hero-title-row"><div><h2>{score>=80?'Locked in.':score>=60?'Good momentum.':'Time to reset.'}</h2><p>Your score combines focus, productive time and distraction control — not just raw screen time.</p><div className="hero-actions"><button className="primary-btn" onClick={onStartFocus}>Start Focus</button><button className="ghost-btn" onClick={()=>onGo('Control')}>Tune limits</button></div></div><div className="score-ring" style={{'--score':`${score}%`} as CSSProperties}><span>{score}</span></div></div></div><div className="stat-card"><span>Tracked today</span><strong>{time(today.tracked)}</strong><small>{minutes(today.productive)}m productive · {minutes(today.distracting)}m distracting</small></div><div className="stat-card"><span>Focus streak</span><strong>{streak}d</strong><small>{streak>0?'Keep the run alive.':'Complete a Focus session today.'}</small></div></section><section className="live-grid"><div className="panel live-panel"><div className="panel-head"><div><span className="section-label">ACTIVE NOW</span><h3>{appName}</h3></div><span className={`live-chip ${data.tracking_paused?'off':''}`}>{data.tracking_paused?'Paused':'Live'}</span></div><div className="live-meta"><span>{data.current_category ?? 'Neutral'}</span><span>{time(data.current_session_secs)} current session</span>{currentRemaining!==null&&<span>{currentRemaining}m left on protection</span>}</div><div className="signal">{[1,2,3,4,5].map(i=><i key={i}/>)}</div></div><div className="panel focus-goal"><span className="section-label">TODAY'S FOCUS</span><strong>{today.focus} / {data.settings.dailyFocusTargetMinutes}m</strong><p>Complete the target to strengthen your Balance score and earn Coins.</p><div className="progress"><span style={{width:`${progress}%`}}/></div></div></section><section className="panel"><div className="panel-head"><div><span className="section-label">USAGE</span><h3>Most-used apps</h3></div><button className="text-btn" onClick={()=>onGo('Insights')}>Open insights →</button></div><div className="app-list">{topApps.length?topApps.slice(0,8).map(app=><div className="app-row" key={app.exe}><div className="app-name"><span className={`category-dot ${app.category.toLowerCase()}`}/><div><strong title={app.exe}>{app.name}</strong><small>{app.category}</small></div></div><div className="app-bar"><span style={{width:`${Math.max(3,(app.secs/topApps[0].secs)*100)}%`}}/></div><strong className="app-time">{time(app.secs)}</strong></div>):<div className="empty">Use the computer for a bit and your real app history will appear here.</div>}</div></section><section className="momentum-strip"><div><span className="section-label">MOMENTUM</span><strong>{today.distracting<=data.settings.dailyDistractionLimitMinutes*60?'Within distraction limit':'Over your distraction target'}</strong><small>Digital Balance updates this in the background while you work.</small></div><button className="ghost-btn" onClick={()=>onGo('Rewards')}>Open Reward Lab</button></section></div>;}

function FocusPage({data,minutes:focusMinutes,remaining,running,completed,focusApps,setFocusApps,knownApps,onDuration,onStart,onPause,onReset,onToggleFullscreen,fullscreen}:{data:TrackerData;minutes:number;remaining:number;running:boolean;completed:boolean;focusApps:string[];setFocusApps:(v:string[])=>void;knownApps:{name:string;exe:string;category:Category;secs:number}[];onDuration:(v:number)=>void;onStart:()=>void;onPause:()=>void;onReset:()=>void;onToggleFullscreen:()=>void;fullscreen:boolean}){
  const mm=String(Math.floor(remaining/60)).padStart(2,'0');
  const ss=String(remaining%60).padStart(2,'0');
  const pct=Math.min(100,Math.max(0,100-(remaining/(focusMinutes*60))*100));
  const phase=pct<18?'dawn':pct<40?'flow':pct<65?'deep':pct<88?'late':'finish';
  const effect=(data.settings.equippedEffect||'effect:clean').replace('effect:','');
  return <div className={`focus-session-page ${running?'is-running':''} effect-${effect}`} data-phase={phase}>
    <div className="focus-ambient" aria-hidden="true">
      <span className="ambient-orb orb-a"/><span className="ambient-orb orb-b"/><span className="ambient-orb orb-c"/><span className="ambient-orb orb-d"/><span className="ambient-vignette"/>
    </div>
    <div className="focus-layout">
      <section className="focus-card">
        <div className="focus-mini-head"><div><span className="section-label">DIGITAL BALANCE · FOCUS</span><strong>{data.settings.mode}</strong><small>{focusApps.length} protected app{focusApps.length===1?'':'s'}</small></div><div className="focus-window-actions"><button className="ghost-btn small" onClick={onToggleFullscreen}>{fullscreen?'Windowed':'Fullscreen'}</button></div></div>
        <span className="focus-phase-label">{running?phase==='dawn'?'SETTLING IN':phase==='flow'?'FIND YOUR RHYTHM':phase==='deep'?'DEEP WORK':phase==='late'?'HOLD THE LINE':'FINAL STRETCH':completed?'SESSION COMPLETE':'READY'}</span>
        <h2>{running?'Stay in the zone.':'Focus without guessing.'}</h2>
        {!running && !completed && <p>{data.settings.mode==='Deep Focus'?'Hard lock-in. Protected apps close the moment the session takes over.':data.settings.mode==='Smart'?'Guided focus. Protected apps close automatically and your layout quiets down.':'Low-friction focus. Selected apps get a ten-minute courtesy window before enforcement.'}</p>}
        <div className="timer-wrap"><div className="timer" style={{'--progress':`${pct}%`} as CSSProperties}><div>{mm}:{ss}</div><small>{running?'Session in progress':completed?'Completed':'Ready'}</small></div></div>
        <div className="duration-row">{[15,25,50,90,120].map(v=><button key={v} disabled={running} className={focusMinutes===v?'selected':''} onClick={()=>onDuration(v)}>{v} min</button>)}</div>
        <div className="focus-actions"><button className="primary-btn" onClick={running?onPause:onStart}>{running?'Pause':remaining===0?'Restart':'Start focus'}</button><button className="ghost-btn" onClick={onReset}>Reset</button></div>
        {completed&&<div className="completion-banner"><Icon name="spark"/> Session secured. Rewards delivered.</div>}
        {!running&&<div className="focus-mode-strip"><span>{effect==='nebula'||effect==='prism'||effect==='golden'?'Premium ambience equipped':'Ambient Focus'}</span><small>Lights breathe and shift through five phases while the session runs.</small></div>}
      </section>
      <section className="focus-side"><div className="panel"><div className="panel-head"><div><span className="section-label">FOCUS PROTECTION</span><h3>Apps to block</h3></div><span className="mini-pill">{focusApps.length} selected</span></div><p className="muted-copy">Deep Focus closes selected processes immediately. Smart does the same with a gentler visual layout. Casual allows a ten-minute grace window before closing.</p><div className="chip-grid">{knownApps.slice(0,18).map(app=><button key={app.exe} className={`app-chip ${focusApps.includes(app.exe)?'selected':''}`} onClick={()=>setFocusApps(focusApps.includes(app.exe)?focusApps.filter(x=>x!==app.exe):[...focusApps,app.exe])}>{app.exe}</button>)}</div></div><div className="panel"><div className="panel-head"><div><span className="section-label">MODE BEHAVIOR</span><h3>{data.settings.mode}</h3></div></div><p className="mode-copy">{data.settings.mode==='Deep Focus'?'Hard friction, optional process blocking, and a distraction-free session HUD.':data.settings.mode==='Smart'?'Gentle nudges, app limits, and context-aware reminders.':'Low-friction monitoring unless you configure explicit protection rules.'}</p></div></section>
    </div>
  </div>;
}

function Insights({data,weekly,topApps}:{data:TrackerData;weekly:{label:string;productive:number;distracting:number;total:number}[];topApps:{name:string;exe:string;category:Category;secs:number}[]}){const max=Math.max(1,...weekly.map(x=>x.total));return <div className="insights-grid"><section className="panel"><div className="panel-head"><div><span className="section-label">LAST 7 DAYS</span><h3>Usage rhythm</h3></div><span className="mini-pill">Live</span></div><div className="week-chart">{weekly.map(day=><div className="week-day" key={day.label}><div className="stack"><i className="productive" style={{height:`${(day.productive/max)*190}px`}}/><i className="distracting" style={{height:`${(day.distracting/max)*190}px`}}/></div><small>{day.label}</small></div>)}</div></section><section className="panel"><span className="section-label">SESSION PATTERNS</span><div className="metric-list"><div><span>Longest session</span><strong>{time(Math.max(0,...data.sessions.map(s=>s.duration_secs)))}</strong></div><div><span>Most-used app</span><strong>{topApps[0]?.name??'No data yet'}</strong></div><div><span>Focus completions</span><strong>{data.focus_sessions.length}</strong></div><div><span>Coins earned</span><strong>{data.focus_sessions.reduce((a,s)=>a+s.coins,0)}</strong></div></div></section><section className="panel"><div className="panel-head"><div><span className="section-label">TOP APPS</span><h3>Where your time goes</h3></div></div><div className="rank-list">{topApps.slice(0,10).map((app,i)=><div key={app.exe}><span className="rank-num">#{String(i+1).padStart(2,'0')}</span><div><strong title={app.exe}>{app.name}</strong><small>{app.category}</small></div><small title={app.exe}>{app.name.toLowerCase()===app.exe.toLowerCase()?'':app.exe}</small><b>{time(app.secs)}</b></div>)}</div></section></div>;}

function ControlPage({data,topApps,onSaveLimit,onRemoveLimit,focusApps,setFocusApps}:{data:TrackerData;topApps:{name:string;exe:string;category:Category;secs:number}[];onSaveLimit:(exe:string,limit:AppLimit)=>Promise<void>;onRemoveLimit:(exe:string)=>Promise<void>;focusApps:string[];setFocusApps:(v:string[])=>void}){
  const [selected,setSelected]=useState(topApps[0]?.exe??'');
  const [limit,setLimit]=useState(data.settings.appLimits[selected]?.dailyMinutes??30);
  const [action,setAction]=useState<LimitAction>(data.settings.appLimits[selected]?.action??'Block');
  const [scope,setScope]=useState<LimitScope>(data.settings.appLimits[selected]?.scope??'Daily');
  const [expiresAt,setExpiresAt]=useState(data.settings.appLimits[selected]?.expiresAt?.slice(0,16)??'');
  useEffect(()=>{const rule=selected?data.settings.appLimits[selected]:undefined;setLimit(rule?.dailyMinutes??30);setAction(rule?.action??'Block');setScope(rule?.scope??'Daily');setExpiresAt(rule?.expiresAt?.slice(0,16)??'');},[selected]);
  const configured=Object.entries(data.settings.appLimits);
  const handleSave=()=>{if(!selected)return;if(scope==='UntilDate'&&!expiresAt){return;}void onSaveLimit(selected,{dailyMinutes:Math.max(1,Math.floor(limit)),action,enabled:true,scope,startedAt:new Date().toISOString(),expiresAt:scope==='UntilDate'?new Date(expiresAt).toISOString():undefined});};
  return <div className="control-grid"><section className="panel"><div className="panel-head"><div><span className="section-label">APP LIMITS</span><h3>Make distraction measurable</h3></div><span className="mini-pill">{configured.length} rules</span></div><p className="muted-copy">Choose any amount of time. Daily limits reset automatically. A date-based rule can run until an exact deadline.</p><div className="limit-builder"><label><span>App</span><SmartSelect value={selected} onChange={setSelected} placeholder="Choose an app" options={[{value:'',label:'Choose an app'},...topApps.map(app=>({value:app.exe,label:app.exe}))]}/></label><label><span>{scope==='Daily'?'Minutes / day':'Total minutes'}</span><input type="number" min="1" step="1" value={limit} onChange={e=>setLimit(Number(e.target.value))}/></label><label><span>Scope</span><SmartSelect value={scope} onChange={v=>setScope(v as LimitScope)} options={[{value:'Daily',label:'Daily'},{value:'UntilDate',label:'Until date'}]}/></label><label><span>Action</span><SmartSelect value={action} onChange={v=>setAction(v as LimitAction)} options={[{value:'Warn',label:'Warn'},{value:'Close',label:'Close'},{value:'Block',label:'Block'}]}/></label>{scope==='UntilDate'&&<label className="limit-date"><span>Deadline</span><input type="datetime-local" value={expiresAt} onChange={e=>setExpiresAt(e.target.value)}/></label>}<div className="quick-times"><button onClick={()=>setLimit(15)}>15m</button><button onClick={()=>setLimit(30)}>30m</button><button onClick={()=>setLimit(60)}>1h</button><button onClick={()=>setLimit(120)}>2h</button></div><button className="primary-btn" disabled={!selected|| (scope==='UntilDate'&&!expiresAt)} onClick={handleSave}>Save protection</button></div><div className="rule-list">{configured.length?configured.map(([exe,rule])=><div className="rule-row" key={exe}><div><strong>{exe}</strong><small>{rule.dailyMinutes.toLocaleString()} min · {rule.scope==='UntilDate'?'until '+new Date(rule.expiresAt??'').toLocaleString(): 'per day'} · {rule.action}</small></div><button className="text-btn danger-text" onClick={()=>void onRemoveLimit(exe)}>Remove</button></div>):<div className="empty">No protection rules yet. Digital Balance itself is never offered as a target.</div>}</div></section><section className="panel"><div className="panel-head"><div><span className="section-label">DEEP FOCUS BLOCKLIST</span><h3>Apps to close during Focus</h3></div><span className="mini-pill">{focusApps.length} selected</span></div><p className="muted-copy">Process-level protection. Select distracting apps and they will be closed while Deep Focus is active.</p><div className="chip-grid">{topApps.slice(0,18).map(app=><button key={app.exe} className={`app-chip ${focusApps.includes(app.exe)?'selected':''}`} onClick={()=>setFocusApps(focusApps.includes(app.exe)?focusApps.filter(x=>x!==app.exe):[...focusApps,app.exe])}>{app.exe}</button>)}</div><div className="active-rule-box"><span>Protected now</span><strong>{configured.length} daily · {focusApps.length} Focus</strong></div></section></div>;}

function RewardsPage({data,rewards:catalog,filter,setFilter,search,setSearch,preview,onPreview,onBuy,onEquip,onApplyTheme,onResetPreview}:{data:TrackerData;rewards:Reward[];filter:'theme'|'frame'|'effect';setFilter:(v:'theme'|'frame'|'effect')=>void;search:string;setSearch:(v:string)=>void;preview:string|null;onPreview:(id:string|null)=>void;onBuy:(r:Reward)=>Promise<void>;onEquip:(r:Reward)=>Promise<void>;onApplyTheme:(id:string)=>Promise<void>;onResetPreview:()=>void}){
  const owned=new Set(data.settings.ownedRewards); const filtered=catalog.filter(r=>r.kind===filter && (!search.trim()||r.name.toLowerCase().includes(search.toLowerCase()))); const tabs=[['theme','Interface themes'],['frame','Profile cosmetics'],['effect','Focus effects']] as const;
  return <div className="rewards-page"><section className="shop-hero"><div><span className="section-label">REWARD LAB</span><h2>Make your workspace yours.</h2><p>Eight free themes start you off. The rest of the collection is earned with Balance Coins, and every theme can be previewed live before you spend.</p></div><div className="coin-card"><span>Balance Coins</span><strong>{data.settings.coins.toLocaleString()}</strong><small>{owned.size} rewards owned</small></div></section><div className="reward-toolbar"><div className="reward-tabs">{tabs.map(([id,label])=><button key={id} className={filter===id?'active':''} onClick={()=>setFilter(id)}>{label} <small>{catalog.filter(r=>r.kind===id).length}</small></button>)}</div><input className="shop-search" value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search the collection…"/>{preview&&<button className="ghost-btn small" onClick={onResetPreview}>Reset preview</button>}</div><div className="theme-live-preview">{preview?<><span className="preview-dot"/><strong>Previewing {themeById(preview).name}</strong><span>No purchase required to preview.</span></>:<><Icon name="spark" size={14}/><span>Hover a theme, then Preview it to see the whole app change live.</span></>}</div><div className="reward-grid">{filtered.map(reward=>{const isOwned=owned.has(reward.id)||reward.price===0;const isEquipped=reward.kind==='theme'?data.settings.theme===reward.id:reward.kind==='frame'?data.settings.equippedFrame===reward.id:data.settings.equippedEffect===reward.id;const insufficient=data.settings.coins<reward.price;return <div key={reward.id} className={`reward-card ${isEquipped?'equipped':''}`} style={{'--reward-accent':reward.accent} as CSSProperties}><div className="reward-art"><span>{reward.kind==='theme'?'◈':reward.kind==='frame'?'◇':'✦'}</span></div><div className="reward-copy"><div className="reward-top"><strong>{reward.name}</strong>{isEquipped&&<em>Equipped</em>}</div><p>{reward.description}</p><div className="reward-bottom"><span>{reward.price===0?'Free':`${reward.price.toLocaleString()} coins`}</span><div className="reward-actions">{reward.kind==='theme'&&<button className="ghost-btn small" onClick={()=>onPreview(reward.id)}>Preview</button>}{isOwned?<button className="primary-btn small" onClick={()=>void (reward.kind==='theme'?onApplyTheme(reward.id):onEquip(reward))}>{isEquipped?'Active':'Equip'}</button>:<button className="primary-btn small" onClick={()=>void onBuy(reward)}>{insufficient?`Need ${(reward.price-data.settings.coins).toLocaleString()}`:'Unlock'}</button>}</div></div></div></div>})}</div></div>;}

function Leaderboard({data,userId,refresh}:{data:LeaderboardRow[];userId:string;refresh:()=>Promise<void>}){return <div className="leaderboard-layout"><section className="panel leader-main"><div className="panel-head"><div><span className="section-label">GLOBAL</span><h3>People building better habits</h3></div><button className="ghost-btn small" onClick={()=>void refresh()}>Refresh</button></div>{data.length?data.slice(0,25).map((row,index)=><div className={`leader-row ${row.user_id===userId?'me':''}`} key={row.user_id}><span className="rank-badge">{String(index+1).padStart(2,'0')}</span><div className="leader-avatar">{row.display_name.slice(0,1).toUpperCase()}</div><div><strong>{row.display_name}</strong><small>{row.user_id===userId?'You':'Digital Balance member'}</small></div><b>{row.total_points.toLocaleString()}</b></div>):<div className="empty">No leaderboard rows yet.</div>}</section><section className="panel leaderboard-side"><span className="section-label">YOUR RANKING</span><div className="big-score">{data.find(r=>r.user_id===userId)?.total_points??0}</div><p>Leaderboard points and Balance Coins are intentionally separate, so shopping never lowers your rank.</p></section></div>;}

function SettingsPage({data,setData,user,onSaveDisplayName}:{data:TrackerData;setData:(data:TrackerData)=>void;user:User|null;onSaveDisplayName:(name:string)=>Promise<void>}){
  const [busy,setBusy]=useState(false); const [name,setName]=useState(data.settings.displayName||user?.user_metadata?.name||user?.email?.split('@')[0]||'Digital Balance');
  useEffect(()=>{setName(data.settings.displayName||user?.user_metadata?.name||user?.email?.split('@')[0]||'Digital Balance');},[data.settings.displayName]);
  async function update(patch:Partial<AppSettings>){setBusy(true);try{setData(await saveSettings({...data.settings,...patch}));}finally{setBusy(false);}}
  const knownApps=[...new Set(data.sessions.map(s=>s.exe))].filter(isProtectedCandidate).sort().slice(0,40);
  return <div className="settings-grid"><section className="panel"><div className="panel-head"><div><span className="section-label">IDENTITY</span><h3>Make the profile yours</h3></div></div><label className="field"><span>Display name</span><input maxLength={28} value={name} onChange={e=>setName(e.target.value)}/></label><div className="name-actions"><button className="primary-btn" onClick={()=>void onSaveDisplayName(name)}>Save name</button><button className="ghost-btn" onClick={()=>{const n=randomName();setName(n);void onSaveDisplayName(n)}}>Random name</button></div><div className="preset-names">{AESTHETIC_NAMES.slice(0,8).map(n=><button key={n} onClick={()=>{setName(n);void onSaveDisplayName(n)}}>{n}</button>)}</div></section><section className="panel"><div className="panel-head"><div><span className="section-label">BEHAVIOR</span><h3>Daily rules</h3></div>{busy&&<span className="mini-pill">Saving…</span>}</div><label className="field"><span>Distraction limit (minutes)</span><input type="number" min="1" value={data.settings.dailyDistractionLimitMinutes} onChange={e=>void update({dailyDistractionLimitMinutes:Math.max(1,Math.floor(Number(e.target.value)||1))})}/></label><label className="field"><span>Focus target (minutes)</span><input type="number" min="1" value={data.settings.dailyFocusTargetMinutes} onChange={e=>void update({dailyFocusTargetMinutes:Math.max(1,Math.floor(Number(e.target.value)||1))})}/></label><label className="field"><span>Idle threshold (seconds)</span><input type="number" min="15" value={data.settings.idleThresholdSeconds} onChange={e=>void update({idleThresholdSeconds:Math.max(15,Math.floor(Number(e.target.value)||15))})}/></label><div className="setting-toggle"><div><strong>Notifications</strong><small>Allow Digital Balance nudges.</small></div><button className={`toggle ${data.settings.notificationsEnabled?'on':''}`} onClick={()=>void update({notificationsEnabled:!data.settings.notificationsEnabled})}><span/></button></div><div className="setting-toggle"><div><strong>Motivational nudges</strong><small>Occasional firm reminders when distraction runs long.</small></div><button className={`toggle ${data.settings.motivationalNudges?'on':''}`} onClick={()=>void update({motivationalNudges:!data.settings.motivationalNudges})}><span/></button></div></section><section className="panel"><div className="panel-head"><div><span className="section-label">WINDOWS</span><h3>Integration</h3></div></div><div className="setting-toggle"><div><strong>Start with Windows</strong><small>Launch the tracker when you sign in.</small></div><button className={`toggle ${data.settings.autostartEnabled?'on':''}`} onClick={()=>void update({autostartEnabled:!data.settings.autostartEnabled})}><span/></button></div><div className="setting-toggle"><div><strong>Ask before closing</strong><small>Offer to keep Digital Balance running quietly in the tray.</small></div><button className={`toggle ${data.settings.closeToTray?'on':''}`} onClick={()=>void update({closeToTray:!data.settings.closeToTray})}><span/></button></div><div className="setting-toggle"><div><strong>Deep Focus auto-block</strong><small>Enforce your Focus blocklist when Deep Focus starts.</small></div><button className={`toggle ${data.settings.deepFocusAutoBlock?'on':''}`} onClick={()=>void update({deepFocusAutoBlock:!data.settings.deepFocusAutoBlock})}><span/></button></div><button className="danger-btn" onClick={async()=>{if(window.confirm('Delete all local Digital Balance activity?'))setData(await deleteLocalData())}}>Delete local activity</button><button className="ghost-btn full" onClick={async()=>{await navigator.clipboard.writeText(await exportLocalData())}}>Copy privacy export JSON</button></section><section className="panel full-width"><div className="panel-head"><div><span className="section-label">CATEGORIES</span><h3>Teach Digital Balance your apps</h3></div><small>Digital Balance and Windows system processes are excluded.</small></div>{knownApps.length?knownApps.map(exe=>{const current=data.settings.categoryOverrides[exe]??data.sessions.find(s=>s.exe===exe)?.category??'Neutral';return <div className="category-row" key={exe}><span>{exe}</span><SmartSelect value={current} onChange={v=>void saveCategory(exe,v).then(setData)} options={categories.map(c=>({value:c,label:c}))} ariaLabel={`Category for ${exe}`}/></div>}):<div className="empty">Use the computer for a bit and app categories will appear here.</div>}</section></div>;}

function CommandPalette({onClose,onGo,onStartFocus}:{onClose:()=>void;onGo:(v:View)=>void;onStartFocus:()=>void}){const actions:[string,View|'focus'][]=[['Open Overview','Overview'],['Open Focus','Focus'],['Open Insights','Insights'],['Open Control Center','Control'],['Open Rewards','Rewards'],['Open Leaderboard','Leaderboard'],['Open Settings','Settings'],['Start Focus now','focus']];return <div className="palette-backdrop" onClick={onClose}><div className="palette" onClick={e=>e.stopPropagation()}><div className="palette-input"><span>⌕</span><input autoFocus placeholder="Jump to a feature…"/></div>{actions.map(([label,target])=><button key={label} onClick={()=>target==='focus'?onStartFocus():onGo(target)}><span>{label}</span><kbd>↵</kbd></button>)}</div></div>;}
function ClosePrompt({onBackground,onQuit}:{onBackground:()=>Promise<void>;onQuit:()=>Promise<void>}){return <div className="modal-backdrop"><div className="close-card"><BrandMark /><span className="section-label">KEEP DIGITAL BALANCE WORKING</span><h2>Run quietly in the background?</h2><p>Closing the window does not have to stop tracking. Digital Balance can hide to the system tray and keep the monitor event-driven with a lightweight periodic check.</p><div className="close-actions"><button className="primary-btn" onClick={()=>void onBackground()}>Keep running in background</button><button className="ghost-btn" onClick={()=>void onQuit()}>Close completely</button></div></div></div>}

