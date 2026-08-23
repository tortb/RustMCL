// CRUD 方法由 M2/M4 阶段的 Tauri command 调用,未全部接线前不告警
#![allow(dead_code)]

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::schema::*;
use crate::error::RunaError;

/// 对 SQLite 连接的 CRUD 封装。方法接受 &Connection,便于在 Tauri state 的 Mutex 中调用。
pub struct Repository;

impl Repository {
    // ---------- instances ----------

    pub fn create_instance(conn: &Connection, inst: &Instance) -> Result<(), RunaError> {
        conn.execute(
            "INSERT INTO instances (id, name, mc_version, loader, loader_version, game_dir, icon_path, created_at, last_played)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                inst.id,
                inst.name,
                inst.mc_version,
                inst.loader,
                inst.loader_version,
                inst.game_dir,
                inst.icon_path,
                inst.created_at,
                inst.last_played,
            ],
        )?;
        Ok(())
    }

    pub fn list_instances(conn: &Connection) -> Result<Vec<Instance>, RunaError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, mc_version, loader, loader_version, game_dir, icon_path, created_at, last_played
             FROM instances ORDER BY COALESCE(last_played, created_at) DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Instance {
                id: row.get(0)?,
                name: row.get(1)?,
                mc_version: row.get(2)?,
                loader: row.get(3)?,
                loader_version: row.get(4)?,
                game_dir: row.get(5)?,
                icon_path: row.get(6)?,
                created_at: row.get(7)?,
                last_played: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_instance(conn: &Connection, id: &str) -> Result<Option<Instance>, RunaError> {
        let inst = conn
            .query_row(
                "SELECT id, name, mc_version, loader, loader_version, game_dir, icon_path, created_at, last_played
                 FROM instances WHERE id = ?1",
                [id],
                |row| {
                    Ok(Instance {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        mc_version: row.get(2)?,
                        loader: row.get(3)?,
                        loader_version: row.get(4)?,
                        game_dir: row.get(5)?,
                        icon_path: row.get(6)?,
                        created_at: row.get(7)?,
                        last_played: row.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(inst)
    }

    pub fn delete_instance(conn: &Connection, id: &str) -> Result<(), RunaError> {
        conn.execute("DELETE FROM instances WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- accounts ----------

    pub fn insert_account(conn: &Connection, acc: &Account) -> Result<(), RunaError> {
        conn.execute(
            "INSERT INTO accounts (id, username, uuid, account_type, is_active, refreshed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                acc.id,
                acc.username,
                acc.uuid,
                acc.account_type,
                acc.is_active as i64,
                acc.refreshed_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_accounts(conn: &Connection) -> Result<Vec<Account>, RunaError> {
        let mut stmt = conn.prepare(
            "SELECT id, username, uuid, account_type, is_active, refreshed_at FROM accounts",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                username: row.get(1)?,
                uuid: row.get(2)?,
                account_type: row.get(3)?,
                is_active: row.get::<_, i64>(4)? != 0,
                refreshed_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 登录成功后写入或更新账号,同时置为当前账号
    pub fn upsert_account(conn: &Connection, acc: &Account) -> Result<(), RunaError> {
        conn.execute(
            "INSERT INTO accounts (id, username, uuid, account_type, is_active, refreshed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               username = excluded.username,
               uuid = excluded.uuid,
               account_type = excluded.account_type,
               is_active = excluded.is_active,
               refreshed_at = excluded.refreshed_at",
            params![
                acc.id,
                acc.username,
                acc.uuid,
                acc.account_type,
                acc.is_active as i64,
                acc.refreshed_at,
            ],
        )?;
        conn.execute("UPDATE accounts SET is_active = 0 WHERE id != ?1", [&acc.id])?;
        Ok(())
    }

    /// 将指定账号置为非当前(退出登录)
    pub fn deactivate_account(conn: &Connection, id: &str) -> Result<(), RunaError> {
        conn.execute("UPDATE accounts SET is_active = 0 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_active_account(conn: &Connection) -> Result<Option<Account>, RunaError> {
        let acc = conn
            .query_row(
                "SELECT id, username, uuid, account_type, is_active, refreshed_at
                 FROM accounts WHERE is_active = 1 LIMIT 1",
                [],
                |row| {
                    Ok(Account {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        uuid: row.get(2)?,
                        account_type: row.get(3)?,
                        is_active: true,
                        refreshed_at: row.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(acc)
    }

    pub fn set_active_account(conn: &Connection, id: &str) -> Result<(), RunaError> {
        conn.execute("UPDATE accounts SET is_active = 0", [])?;
        conn.execute("UPDATE accounts SET is_active = 1 WHERE id = ?1", [id])?;
        Ok(())
    }
}
