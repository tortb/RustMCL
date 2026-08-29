pub mod migrations;
pub mod repository;
pub mod schema;

use std::path::Path;

use rusqlite::Connection;

use crate::error::RmclError;

/// 打开数据库连接并应用 migration
pub fn init(path: &Path) -> Result<Connection, RmclError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run(&conn)?;
    Ok(conn)
}
