-- Digital Balance Supabase schema
-- Existing project may already have this. Safe to run only if tables/functions do not already exist.

create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  username text unique not null,
  display_name text not null,
  points integer not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists profiles_points_idx on public.profiles(points desc);
alter table public.profiles enable row level security;

drop policy if exists "Anyone can view leaderboard" on public.profiles;
create policy "Anyone can view leaderboard" on public.profiles for select to authenticated using (true);

drop policy if exists "Users can create own profile" on public.profiles;
create policy "Users can create own profile" on public.profiles for insert to authenticated with check (auth.uid() = id);

drop policy if exists "Users can update own profile" on public.profiles;
create policy "Users can update own profile" on public.profiles for update to authenticated using (auth.uid() = id) with check (auth.uid() = id);

create or replace function public.handle_new_user()
returns trigger language plpgsql security definer set search_path = public as $$
begin
  insert into public.profiles (id, username, display_name)
  values (
    new.id,
    'user_' || substr(replace(new.id::text, '-', ''), 1, 8),
    coalesce(new.raw_user_meta_data ->> 'name', 'Digital Balance User')
  ) on conflict (id) do nothing;
  return new;
end;
$$;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created after insert on auth.users for each row execute function public.handle_new_user();

create or replace function public.ensure_profile(p_display_name text)
returns public.profiles language plpgsql security definer set search_path = public as $$
declare result public.profiles;
begin
  if auth.uid() is null then raise exception 'Not authenticated'; end if;
  insert into public.profiles (id, username, display_name)
  values (auth.uid(), 'user_' || substr(replace(auth.uid()::text, '-', ''), 1, 8), coalesce(nullif(trim(p_display_name), ''), 'Digital Balance User'))
  on conflict (id) do update set display_name = excluded.display_name, updated_at = now()
  returning * into result;
  return result;
end;
$$;

create or replace function public.award_focus_points(p_minutes integer)
returns integer language plpgsql security definer set search_path = public as $$
declare awarded integer;
begin
  if auth.uid() is null then raise exception 'Not authenticated'; end if;
  if p_minutes is null or p_minutes <= 0 then return 0; end if;
  awarded := least(p_minutes, 120);
  update public.profiles set points = points + awarded, updated_at = now() where id = auth.uid();
  return awarded;
end;
$$;

revoke all on function public.ensure_profile(text) from public;
grant execute on function public.ensure_profile(text) to authenticated;
revoke all on function public.award_focus_points(integer) from public;
grant execute on function public.award_focus_points(integer) to authenticated;
