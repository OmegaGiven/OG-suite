alter table users add column if not exists username text;
alter table users add column if not exists password_hash text;
alter table users add column if not exists must_change_password boolean not null default false;
alter table users add column if not exists storage_limit_mb bigint not null default 2048;
alter table users add column if not exists app_scopes_json jsonb not null default '{}'::jsonb;
alter table users add column if not exists updated_at timestamptz not null default now();

create unique index if not exists users_username_idx on users (lower(username)) where username is not null;

alter table workspaces add column if not exists owner_id uuid references users(id);
alter table workspaces add column if not exists updated_at timestamptz not null default now();

create table if not exists auth_sessions (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  workspace_id uuid not null references workspaces(id) on delete cascade,
  access_token_hash text not null unique,
  refresh_token_hash text not null unique,
  expires_at timestamptz not null,
  revoked_at timestamptz,
  created_at timestamptz not null default now()
);

create index if not exists auth_sessions_user_idx on auth_sessions (user_id);
create index if not exists auth_sessions_expires_at_idx on auth_sessions (expires_at);

create table if not exists trusted_devices (
  id uuid primary key,
  user_id uuid not null references users(id) on delete cascade,
  device_id text not null,
  device_name text not null,
  platform text not null,
  created_at timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  unique (user_id, device_id)
);

create table if not exists appearance_themes (
  id uuid primary key,
  name text not null,
  tokens jsonb not null,
  owner_id text not null,
  workspace_id text not null,
  is_shared boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists appearance_themes_workspace_idx on appearance_themes (workspace_id, is_shared);
create index if not exists appearance_themes_owner_idx on appearance_themes (owner_id);

create table if not exists appearance_settings (
  user_id text not null,
  workspace_id text not null,
  tokens jsonb not null,
  updated_at timestamptz not null default now(),
  primary key (user_id, workspace_id)
);

create index if not exists appearance_settings_workspace_idx on appearance_settings (workspace_id);
