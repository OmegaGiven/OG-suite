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
