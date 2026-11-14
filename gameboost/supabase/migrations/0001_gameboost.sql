-- GameBoost: Datenbank-Schema für Konten und Cloud-Sync.
-- Im Supabase-Dashboard unter „SQL Editor“ einfügen und ausführen.

-- Profil je Nutzer (vorbereitet für Free/Pro)
create table if not exists public.profiles (
  user_id uuid primary key references auth.users (id) on delete cascade,
  display_name text,
  plan text not null default 'free' check (plan in ('free', 'pro')),
  created_at timestamptz not null default now()
);

-- Synchronisierte Einstellungen (nur geräteunabhängige Teile)
create table if not exists public.user_settings (
  user_id uuid primary key references auth.users (id) on delete cascade,
  settings jsonb not null,
  updated_at timestamptz not null default now()
);

-- Vorher-/Nachher-Messungen
create table if not exists public.sessions (
  user_id uuid not null references auth.users (id) on delete cascade,
  id text not null,
  created_at timestamptz not null,
  data jsonb not null,
  primary key (user_id, id)
);

-- Zeilenbasierte Sicherheit: jeder sieht und ändert nur seine eigenen Daten
alter table public.profiles enable row level security;
alter table public.user_settings enable row level security;
alter table public.sessions enable row level security;

create policy "eigenes Profil lesen" on public.profiles for select using (auth.uid() = user_id);
create policy "eigenes Profil ändern" on public.profiles for update using (auth.uid() = user_id) with check (auth.uid() = user_id);
-- Den Tarif (Free/Pro) darf nur der Server ändern, nie die App selbst
revoke update (plan) on public.profiles from authenticated, anon;

create policy "eigene Einstellungen" on public.user_settings for all using (auth.uid() = user_id) with check (auth.uid() = user_id);
create policy "eigene Messungen" on public.sessions for all using (auth.uid() = user_id) with check (auth.uid() = user_id);

-- Profil automatisch anlegen, wenn sich jemand registriert
create or replace function public.handle_new_user() returns trigger
language plpgsql security definer set search_path = public as $$
begin
  insert into public.profiles (user_id, display_name)
  values (new.id, coalesce(new.raw_user_meta_data ->> 'full_name', new.raw_user_meta_data ->> 'name', split_part(new.email, '@', 1)))
  on conflict (user_id) do nothing;
  return new;
end;
$$;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created after insert on auth.users
  for each row execute function public.handle_new_user();
