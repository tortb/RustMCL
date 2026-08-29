-- M1 服务器列表:对应模块 1.1
CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    port INTEGER NOT NULL,
    is_favorite INTEGER DEFAULT 0,
    icon_base64 TEXT,
    last_ping_ms INTEGER,
    sort_order INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_servers_order ON servers(sort_order);
