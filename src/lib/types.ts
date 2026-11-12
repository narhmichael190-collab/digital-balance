export type AppMode = 'Casual' | 'Smart' | 'Deep Focus';
export type Category = 'Productive' | 'Distracting' | 'Communication' | 'Media' | 'Neutral';
export type LimitAction = 'Warn' | 'Close' | 'Block';
export type LimitScope = 'Daily' | 'UntilDate';

export type AppLimit = {
  dailyMinutes: number;
  action: LimitAction;
  enabled: boolean;
  scope?: LimitScope;
  startedAt?: string;
  expiresAt?: string;
};

export type AppSettings = {
  dailyDistractionLimitMinutes: number;
  dailyFocusTargetMinutes: number;
  idleThresholdSeconds: number;
  mode: AppMode;
  notificationsEnabled: boolean;
  autostartEnabled: boolean;
  closeToTray: boolean;
  motivationalNudges: boolean;
  displayName: string;
  categoryOverrides: Record<string, Category>;
  appLimits: Record<string, AppLimit>;
  theme: string;
  purchasedThemes: string[];
  coins: number;
  ownedRewards: string[];
  equippedFrame: string;
  equippedEffect: string;
  deepFocusBlockedApps: string[];
  deepFocusAutoBlock: boolean;
};

export type SessionRecord = {
  app_name: string;
  exe: string;
  category: Category;
  started_at: string;
  ended_at: string;
  duration_secs: number;
};

export type FocusSession = {
  minutes: number;
  points: number;
  coins: number;
  completed_at: string;
  mode: AppMode;
};

export type TrackerData = {
  sessions: SessionRecord[];
  focus_sessions: FocusSession[];
  current_app: string | null;
  current_exe: string | null;
  current_category: Category | null;
  current_session_secs: number;
  tracking_paused: boolean;
  settings: AppSettings;
};

export type LeaderboardRow = {
  user_id: string;
  display_name: string;
  total_points: number;
  created_at: string;
};

export type Reward = {
  id: string;
  name: string;
  description: string;
  price: number;
  kind: 'theme' | 'frame' | 'effect';
  accent: string;
};
