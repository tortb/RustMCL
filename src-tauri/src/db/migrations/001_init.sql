-- M0 初始 schema:对应规格 4.1
CREATE TABLE IF NOT EXISTS instances (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    mc_version TEXT NOT NULL,
    loader TEXT,
    loader_version TEXT,
    game_dir TEXT NOT NULL,
    icon_path TEXT,
    created_at INTEGER NOT NULL,
    last_played INTEGER
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    uuid TEXT NOT NULL,
    account_type TEXT NOT NULL,
    is_active INTEGER DEFAULT 0,
    refreshed_at INTEGER
);

CREATE TABLE IF NOT EXISTS mods (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    source TEXT,
    project_id TEXT,
    version_id TEXT,
    enabled INTEGER DEFAULT 1
);

CREATE TABLE IF NOT EXISTS asset_cache (
    sha1 TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    size INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_instances_last_played ON instances(last_played);
CREATE INDEX IF NOT EXISTS idx_mods_instance ON mods(instance_id);
