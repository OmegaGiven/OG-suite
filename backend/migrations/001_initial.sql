create table if not exists users (
  id uuid primary key,
  display_name text not null,
  roles jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now()
);

create table if not exists workspaces (
  id uuid primary key,
  name text not null,
  created_at timestamptz not null default now()
);

create table if not exists app_registry (
  id text primary key,
  name text not null,
  route text not null,
  standalone_route text not null,
  capabilities jsonb not null default '[]'::jsonb
);

create table if not exists crdt_documents (
  id uuid primary key,
  kind text not null,
  snapshot text not null default '',
  version bigint not null default 0,
  compacted_at timestamptz
);

create table if not exists crdt_document_updates (
  id uuid primary key,
  document_id uuid not null references crdt_documents(id) on delete cascade,
  client_id text not null,
  sequence bigint not null,
  payload text not null,
  created_at timestamptz not null default now()
);

create table if not exists notes (
  id uuid primary key,
  document_id uuid not null references crdt_documents(id) on delete cascade,
  title text not null,
  path text not null,
  tags jsonb not null default '[]'::jsonb,
  owner_id text not null,
  workspace_id text not null,
  created_at timestamptz not null,
  updated_at timestamptz not null,
  deleted_at timestamptz
);

create table if not exists note_folders (
  id uuid primary key,
  path text not null,
  name text not null,
  owner_id text not null,
  workspace_id text not null,
  created_at timestamptz not null,
  updated_at timestamptz not null,
  deleted_at timestamptz
);

create table if not exists sync_cursors (
  id text primary key,
  generated_at timestamptz not null
);

create table if not exists sync_tombstones (
  entity text not null,
  id uuid not null,
  deleted_at timestamptz not null,
  primary key (entity, id)
);

create table if not exists feed_activity_events (
  id uuid primary key,
  app_id text not null,
  action text not null,
  summary text not null,
  target_kind text not null,
  target_id text not null,
  target_label text not null,
  actor_id text not null,
  actor_name text not null,
  workspace_id text not null,
  is_public boolean not null default true,
  created_at timestamptz not null
);

create index if not exists feed_activity_events_created_at_idx on feed_activity_events (created_at desc);

create table if not exists feed_favorites (
  id uuid primary key,
  target_kind text not null,
  target_id text not null,
  label text not null,
  app_id text not null,
  actor_id text not null,
  workspace_id text not null,
  created_at timestamptz not null,
  unique (target_kind, target_id, actor_id, workspace_id)
);

create table if not exists audio_recordings (
  id uuid primary key,
  title text not null,
  path text not null default '/',
  mime_type text not null,
  duration_ms bigint not null default 0,
  size_bytes bigint not null default 0,
  status text not null,
  asset_ref text,
  owner_id text not null,
  workspace_id text not null,
  created_at timestamptz not null,
  updated_at timestamptz not null,
  deleted_at timestamptz
);

create table if not exists audio_folders (
  id uuid primary key,
  path text not null,
  name text not null,
  owner_id text not null,
  workspace_id text not null,
  created_at timestamptz not null,
  updated_at timestamptz not null,
  deleted_at timestamptz
);

create unique index if not exists audio_folders_active_path_idx on audio_folders (path) where deleted_at is null;

create table if not exists audio_transcript_segments (
  id uuid primary key,
  recording_id uuid not null references audio_recordings(id) on delete cascade,
  channel integer,
  speaker_label text,
  start_ms bigint not null,
  end_ms bigint not null,
  text text not null
);
