use rusqlite::Connection;

use crate::error::RmclError;

/// 当前 schema 版本,与 migrations/ 目录下的文件一一对应
const CURRENT_VERSION: i64 = 3;

/// 启动时按需执行未应用的 migration,通过 PRAGMA user_version 追踪版本
pub fn run(conn: &Connection) -> Result<(), RmclError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 1 {
        conn.execute_batch(include_str!("001_init.sql"))?;
    }
    if version < 2 {
        conn.execute_batch(include_str!("002_servers.sql"))?;
    }
    if version < 3 {
        conn.execute_batch(include_str!("003_resource_packs.sql"))?;
    }
    if version < CURRENT_VERSION {
        conn.pragma_update(None, "user_version", CURRENT_VERSION)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent_and_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        run(&conn).unwrap(); // 再次执行不应报错

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('instances','accounts','mods','asset_cache','servers','resource_packs')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 6);
    }
}
