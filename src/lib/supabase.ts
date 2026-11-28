import { createClient, type SupabaseClient, type User } from '@supabase/supabase-js';
import { openUrl } from '@tauri-apps/plugin-opener';
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';

const url = import.meta.env.VITE_SUPABASE_URL?.trim() ?? '';
const key = (import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY?.trim() || import.meta.env.VITE_SUPABASE_ANON_KEY?.trim() || '');

export const supabaseConfigIssue = !url
  ? 'Missing VITE_SUPABASE_URL in .env'
  : !key
    ? 'Missing VITE_SUPABASE_PUBLISHABLE_KEY (or VITE_SUPABASE_ANON_KEY) in .env'
    : url.includes('YOUR_PROJECT')
      ? 'VITE_SUPABASE_URL still contains the example placeholder'
      : null;

export const supabaseConfigured = supabaseConfigIssue === null;
export const supabase: SupabaseClient | null = supabaseConfigured
  ? createClient(url, key, {
      auth: {
        flowType: 'pkce',
        detectSessionInUrl: false,
        persistSession: true,
        autoRefreshToken: true,
      },
    })
  : null;

export async function signInWithAzure() {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const { data, error } = await supabase.auth.signInWithOAuth({
    provider: 'azure',
    options: {
      redirectTo: 'digitalbalance://auth/callback',
      skipBrowserRedirect: true,
      scopes: 'email',
    },
  });
  if (error) throw error;
  if (data.url) await openUrl(data.url);
}

export async function signInWithEmailPassword(email: string, password: string) {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const cleanEmail = email.trim();
  if (!cleanEmail || !password) throw new Error('Enter your email and password.');
  const { error } = await supabase.auth.signInWithPassword({ email: cleanEmail, password });
  if (error) throw error;
}

export async function signUpWithEmailPassword(email: string, password: string) {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const cleanEmail = email.trim();
  if (!cleanEmail || !password) throw new Error('Enter your email and password.');
  if (password.length < 8) throw new Error('Use a password with at least 8 characters.');
  const { data, error } = await supabase.auth.signUp({ email: cleanEmail, password });
  if (error) throw error;
  return data;
}

export async function resetPasswordForEmail(email: string) {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const cleanEmail = email.trim();
  if (!cleanEmail) throw new Error('Enter your email first.');
  const { error } = await supabase.auth.resetPasswordForEmail(cleanEmail, {
    redirectTo: 'digitalbalance://auth/callback',
  });
  if (error) throw error;
}

export async function updatePassword(password: string) {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  if (password.length < 8) throw new Error('Use a password with at least 8 characters.');
  const { error } = await supabase.auth.updateUser({ password });
  if (error) throw error;
}

export async function signInWithGoogle() {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const { data, error } = await supabase.auth.signInWithOAuth({
    provider: 'google',
    options: { redirectTo: 'digitalbalance://auth/callback', skipBrowserRedirect: true },
  });
  if (error) throw error;
  if (data.url) await openUrl(data.url);
}

async function consumeAuthUrl(rawUrl: string) {
  if (!supabase) return;
  const parsed = new URL(rawUrl);
  const code = parsed.searchParams.get('code');
  if (code) {
    const { error } = await supabase.auth.exchangeCodeForSession(code);
    if (error) console.error('OAuth exchange failed', error);
    return;
  }
  const hash = new URLSearchParams(parsed.hash.replace(/^#/, ''));
  const access_token = hash.get('access_token');
  const refresh_token = hash.get('refresh_token');
  if (access_token && refresh_token) {
    const { error } = await supabase.auth.setSession({ access_token, refresh_token });
    if (error) console.error('Session restore failed', error);
  }
}

export async function setupDeepLinkAuth() {
  if (!supabase) return () => undefined;
  try {
    const current = await getCurrent();
    for (const value of current ?? []) await consumeAuthUrl(value);
  } catch (error) {
    console.warn('Deep link startup check failed', error);
  }
  const unlisten = await onOpenUrl(async (urls) => {
    for (const value of urls) await consumeAuthUrl(value);
  });
  return unlisten;
}

export type Profile = {
  id: string;
  username: string;
  display_name: string;
  points?: number;
  streak?: number;
  created_at?: string;
  updated_at?: string;
  avatar_path?: string | null;
  avatar_url?: string | null;
};

export async function ensureProfile(user: User) {
  if (!supabase) return;
  const { data: existing, error: readError } = await supabase
    .from('profiles')
    .select('id')
    .eq('id', user.id)
    .maybeSingle();
  if (readError) throw readError;
  if (existing) return;

  const displayName = user.user_metadata?.full_name || user.user_metadata?.name || user.email?.split('@')[0] || 'Liba Bilancia user';
  const { error } = await supabase.from('profiles').insert({
    id: user.id,
    username: `user_${user.id.replaceAll('-', '').slice(0, 8)}`,
    display_name: displayName,
  });
  if (error) throw error;
}

async function signedAvatarUrl(path: string | null | undefined) {
  if (!supabase || !path) return null;
  const { data, error } = await supabase.storage
    .from('profile-pictures')
    .createSignedUrl(path, 60 * 60 * 24);
  if (error) throw error;
  return data?.signedUrl ?? null;
}

export async function getCurrentProfile(): Promise<Profile | null> {
  if (!supabase) return null;
  const { data: auth } = await supabase.auth.getUser();
  if (!auth.user) return null;
  const { data, error } = await supabase
    .from('profiles')
    .select('id, username, display_name, points, streak, created_at, updated_at, avatar_path')
    .eq('id', auth.user.id)
    .maybeSingle();
  if (error) throw error;
  if (!data) return null;
  return { ...data, avatar_url: await signedAvatarUrl(data.avatar_path) } as Profile;
}

export async function uploadProfileAvatar(blob: Blob) {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const { data: auth } = await supabase.auth.getUser();
  if (!auth.user) throw new Error('You are not signed in.');
  if (blob.size > 5 * 1024 * 1024) throw new Error('That image is still too large. Please choose a smaller picture.');

  const path = `${auth.user.id}/avatar.webp`;
  const { error: uploadError } = await supabase.storage
    .from('profile-pictures')
    .upload(path, blob, {
      cacheControl: '86400',
      contentType: 'image/webp',
      upsert: true,
    });
  if (uploadError) throw uploadError;

  const { error: profileError } = await supabase
    .from('profiles')
    .update({ avatar_path: path, updated_at: new Date().toISOString() })
    .eq('id', auth.user.id);
  if (profileError) throw profileError;

  return await signedAvatarUrl(path);
}

export async function removeProfileAvatar() {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const { data: auth } = await supabase.auth.getUser();
  if (!auth.user) throw new Error('You are not signed in.');
  const path = `${auth.user.id}/avatar.webp`;
  const { error: storageError } = await supabase.storage.from('profile-pictures').remove([path]);
  if (storageError) throw storageError;
  const { error: profileError } = await supabase
    .from('profiles')
    .update({ avatar_path: null, updated_at: new Date().toISOString() })
    .eq('id', auth.user.id);
  if (profileError) throw profileError;
}

export async function isUsernameAvailable(username: string) {
  if (!supabase) return false;
  const clean = username.trim().toLowerCase();
  if (!/^[a-z0-9_]{3,20}$/.test(clean)) return false;
  const { data: auth } = await supabase.auth.getUser();
  if (!auth.user) return false;

  const { data, error } = await supabase
    .from('profiles')
    .select('id')
    .ilike('username', clean)
    .neq('id', auth.user.id)
    .limit(1);

  if (error) throw error;
  return (data?.length ?? 0) === 0;
}

export async function updateProfileUsername(username: string) {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const clean = username.trim().toLowerCase();

  if (!/^[a-z0-9_]{3,20}$/.test(clean)) {
    throw new Error('Username must be 3–20 characters using lowercase letters, numbers, or underscores.');
  }

  const { data: auth } = await supabase.auth.getUser();
  if (!auth.user) throw new Error('You are not signed in.');

  const { error } = await supabase
    .from('profiles')
    .update({
      username: clean,
      display_name: clean,
      updated_at: new Date().toISOString(),
    })
    .eq('id', auth.user.id);

  if (error) {
    if (error.code === '23505') throw new Error('That username is already taken.');
    throw error;
  }
}


export const CURRENT_SEASON_ID = 'season-01-first-light';
export const CURRENT_SEASON_GOAL = 5000;

async function recordSeasonPoints(points: number) {
  if (!supabase || !Number.isFinite(points) || points <= 0) return;
  const { error } = await supabase.rpc('record_season_points', {
    p_season_id: CURRENT_SEASON_ID,
    p_points: Math.round(points),
  });
  if (error) {
    // Seasonal tracking must never block the existing verified points reward.
    console.warn('Could not record Season Points. Apply the Season 01 SQL setup:', error);
  }
}

export type SeasonProgress = { points: number; rewardClaimed: boolean };
export async function fetchCurrentSeasonProgress(): Promise<SeasonProgress> {
  if (!supabase) return { points: 0, rewardClaimed: false };
  const { data: auth, error: authError } = await supabase.auth.getUser();
  if (authError) throw authError;
  if (!auth.user) return { points: 0, rewardClaimed: false };
  const { data, error } = await supabase
    .from('season_progress')
    .select('season_points, reward_claimed')
    .eq('user_id', auth.user.id)
    .eq('season_id', CURRENT_SEASON_ID)
    .maybeSingle();
  if (error) throw error;
  return {
    points: Number(data?.season_points ?? 0),
    rewardClaimed: Boolean(data?.reward_claimed),
  };
}

export async function claimCurrentSeasonReward(): Promise<{ claimed: boolean; alreadyClaimed: boolean; coins: number }> {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const { data, error } = await supabase.rpc('claim_season_reward', { p_season_id: CURRENT_SEASON_ID });
  if (error) throw error;
  const result = (data ?? {}) as Record<string, unknown>;
  return {
    claimed: Boolean(result.claimed),
    alreadyClaimed: Boolean(result.already_claimed),
    coins: Number(result.coins ?? 0),
  };
}

export type ActivityRewardResult = { points: number; coins: number; claimed: boolean };

export async function claimActivityReward(action: 'daily_checkin' | 'daily_focus_goal' | 'three_focus_sessions' | 'weekly_consistency'): Promise<ActivityRewardResult> {
  if (!supabase) return { points: 0, coins: 0, claimed: false };
  const { data, error } = await supabase.rpc('claim_activity_reward', { p_action: action });
  if (error) throw error;
  const row = (data ?? {}) as Record<string, unknown>;
  const result = {
    points: Number(row.points ?? 0),
    coins: Number(row.coins ?? 0),
    claimed: Boolean(row.claimed),
  };
  if (result.claimed && result.points > 0) await recordSeasonPoints(result.points);
  return result;
}

export async function updateProfileDisplayName(displayName: string) {
  if (!supabase) return;
  const clean = displayName.trim().slice(0, 28);
  if (!clean) throw new Error('Choose a display name first.');
  const { data: auth } = await supabase.auth.getUser();
  if (!auth.user) throw new Error('You are not signed in.');
  const { error } = await supabase
    .from('profiles')
    .update({ display_name: clean, updated_at: new Date().toISOString() })
    .eq('id', auth.user.id);
  if (error) throw error;
}

export async function fetchLeaderboard() {
  if (!supabase) return [];
  const { data, error } = await supabase
    .from('profiles')
    .select('user_id:id, display_name, total_points:points, created_at, avatar_path')
    .order('points', { ascending: false })
    .limit(50);
  if (error) throw error;

  return await Promise.all((data ?? []).map(async (row) => ({
    ...row,
    avatar_url: await signedAvatarUrl(row.avatar_path),
  })));
}

export async function awardFocusPoints(minutes: number) {
  if (!supabase) return { points: 0 };
  const { data, error } = await supabase.rpc('award_focus_points', { p_minutes: Math.round(minutes) });
  if (error) throw error;
  const points = Number(data ?? 0);
  await recordSeasonPoints(points);
  return { points };
}


export type FeedbackCategory = 'bug' | 'concern' | 'suggestion' | 'general';

export async function submitFeedback(category: FeedbackCategory, message: string) {
  if (!supabase) throw new Error('Supabase is not configured yet.');
  const clean = message.trim();
  if (clean.length < 5) throw new Error('Please write a little more so the feedback is useful.');
  if (clean.length > 2000) throw new Error('Feedback must be 2000 characters or fewer.');
  const allowed: FeedbackCategory[] = ['bug', 'concern', 'suggestion', 'general'];
  if (!allowed.includes(category)) throw new Error('Choose a valid feedback type.');

  const { data, error } = await supabase.functions.invoke('submit-feedback', {
    body: { category, message: clean },
  });

  if (error) throw error;
  if (!data?.ok) throw new Error('Feedback could not be submitted.');
  return data;
}
