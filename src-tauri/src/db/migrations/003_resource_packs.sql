-- M4 资源包/光影包:对应模块 4.1
CREATE TABLE IF NOT EXISTS resource_packs (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    file_name TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rp_instance ON resource_packs(instance_id);
