// CRUD 方法由 M2/M4 阶段的 Tauri command 调用,未全部接线前不告警
#![allow(dead_code)]

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::schema::*;
use crate::error::RmclError;

/// 对 SQLite 连接的 CRUD 封装。方法接受 &Connection,便于在 Tauri state 的 Mutex 中调用。
pub struct Repository;

impl Repository {
    // ---------- instances ----------

    pub fn create_instance(conn: &Connection, inst: &Instance) -> Result<(), RmclError> {
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

    pub fn list_instances(conn: &Connection) -> Result<Vec<Instance>, RmclError> {
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

    pub fn get_instance(conn: &Connection, id: &str) -> Result<Option<Instance>, RmclError> {
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

    pub fn update_instance(conn: &Connection, inst: &Instance) -> Result<(), RmclError> {
        conn.execute(
            "UPDATE instances SET name = ?1, mc_version = ?2, loader = ?3, loader_version = ?4
             WHERE id = ?5",
            params![
                inst.name,
                inst.mc_version,
                inst.loader,
                inst.loader_version,
                inst.id,
            ],
        )?;
        Ok(())
    }

    pub fn delete_instance(conn: &Connection, id: &str) -> Result<(), RmclError> {
        conn.execute("DELETE FROM instances WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- accounts ----------

    pub fn insert_account(conn: &Connection, acc: &Account) -> Result<(), RmclError> {
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

    pub fn list_accounts(conn: &Connection) -> Result<Vec<Account>, RmclError> {
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
    pub fn upsert_account(conn: &Connection, acc: &Account) -> Result<(), RmclError> {
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
    pub fn deactivate_account(conn: &Connection, id: &str) -> Result<(), RmclError> {
        conn.execute("UPDATE accounts SET is_active = 0 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_active_account(conn: &Connection) -> Result<Option<Account>, RmclError> {
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

    pub fn set_active_account(conn: &Connection, id: &str) -> Result<(), RmclError> {
        conn.execute("UPDATE accounts SET is_active = 0", [])?;
        conn.execute("UPDATE accounts SET is_active = 1 WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- mods ----------

    pub fn insert_mod(conn: &Connection, m: &ModEntry) -> Result<(), RmclError> {
        conn.execute(
            "INSERT INTO mods (id, instance_id, file_name, source, project_id, version_id, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                m.id,
                m.instance_id,
                m.file_name,
                m.source,
                m.project_id,
                m.version_id,
                m.enabled as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_mods(conn: &Connection, instance_id: &str) -> Result<Vec<ModEntry>, RmclError> {
        let mut stmt = conn.prepare(
            "SELECT id, instance_id, file_name, source, project_id, version_id, enabled
             FROM mods WHERE instance_id = ?1 ORDER BY file_name",
        )?;
        let rows = stmt.query_map([instance_id], |row| {
            Ok(ModEntry {
                id: row.get(0)?,
                instance_id: row.get(1)?,
                file_name: row.get(2)?,
                source: row.get(3)?,
                project_id: row.get(4)?,
                version_id: row.get(5)?,
                enabled: row.get::<_, i64>(6)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_mod(conn: &Connection, id: &str) -> Result<Option<ModEntry>, RmclError> {
        let m = conn
            .query_row(
                "SELECT id, instance_id, file_name, source, project_id, version_id, enabled
                 FROM mods WHERE id = ?1",
                [id],
                |row| {
                    Ok(ModEntry {
                        id: row.get(0)?,
                        instance_id: row.get(1)?,
                        file_name: row.get(2)?,
                        source: row.get(3)?,
                        project_id: row.get(4)?,
                        version_id: row.get(5)?,
                        enabled: row.get::<_, i64>(6)? != 0,
                    })
                },
            )
            .optional()?;
        Ok(m)
    }

    pub fn set_mod_enabled(conn: &Connection, id: &str, enabled: bool) -> Result<(), RmclError> {
        conn.execute("UPDATE mods SET enabled = ?1 WHERE id = ?2", params![enabled as i64, id])?;
        Ok(())
    }

    pub fn delete_mod(conn: &Connection, id: &str) -> Result<(), RmclError> {
        conn.execute("DELETE FROM mods WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------- servers ----------

    pub fn insert_server(conn: &Connection, s: &ServerEntry) -> Result<(), RmclError> {
        conn.execute(
            "INSERT INTO servers (id, name, address, port, is_favorite, icon_base64, last_ping_ms, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                s.id,
                s.name,
                s.address,
                s.port as i64,
                s.is_favorite as i64,
                s.icon_base64,
                s.last_ping_ms,
                s.sort_order,
                s.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_servers(conn: &Connection) -> Result<Vec<ServerEntry>, RmclError> {
        let mut stmt = conn.prepare(
            "SELECT id, name, address, port, is_favorite, icon_base64, last_ping_ms, sort_order, created_at
             FROM servers ORDER BY is_favorite DESC, sort_order ASC, name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ServerEntry {
                id: row.get(0)?,
                name: row.get(1)?,
                address: row.get(2)?,
                port: row.get::<_, i64>(3)? as u16,
                is_favorite: row.get::<_, i64>(4)? != 0,
                icon_base64: row.get(5)?,
                last_ping_ms: row.get(6)?,
                sort_order: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_server(conn: &Connection, id: &str) -> Result<Option<ServerEntry>, RmclError> {
        let s = conn
            .query_row(
                "SELECT id, name, address, port, is_favorite, icon_base64, last_ping_ms, sort_order, created_at
                 FROM servers WHERE id = ?1",
                [id],
                |row| {
                    Ok(ServerEntry {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        address: row.get(2)?,
                        port: row.get::<_, i64>(3)? as u16,
                        is_favorite: row.get::<_, i64>(4)? != 0,
                        icon_base64: row.get(5)?,
                        last_ping_ms: row.get(6)?,
                        sort_order: row.get(7)?,
                        created_at: row.get(8)?,
                    })
                },
            )
            .optional()?;
        Ok(s)
    }

    pub fn delete_server(conn: &Connection, id: &str) -> Result<(), RmclError> {
        conn.execute("DELETE FROM servers WHERE id = ?1", [id])?;
        Ok(())
    }

    /// 更新服务器(仅更新传入的 Some 字段)
    pub fn update_server(conn: &Connection, id: &str, name: Option<&str>, favorite: Option<bool>, sort_order: Option<i64>) -> Result<(), RmclError> {
        if let Some(name) = name {
            conn.execute("UPDATE servers SET name = ?1 WHERE id = ?2", params![name, id])?;
        }
        if let Some(fav) = favorite {
            conn.execute("UPDATE servers SET is_favorite = ?1 WHERE id = ?2", params![fav as i64, id])?;
        }
        if let Some(so) = sort_order {
            conn.execute("UPDATE servers SET sort_order = ?1 WHERE id = ?2", params![so, id])?;
        }
        Ok(())
    }

    pub fn set_server_ping(conn: &Connection, id: &str, ping_ms: i64) -> Result<(), RmclError> {
        conn.execute("UPDATE servers SET last_ping_ms = ?1 WHERE id = ?2", params![ping_ms, id])?;
        Ok(())
    }

    // ---------- resource_packs ----------

    pub fn upsert_resource_pack(conn: &Connection, p: &ResourcePackEntry) -> Result<(), RmclError> {
        conn.execute(
            "INSERT INTO resource_packs (id, instance_id, type, file_name, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET file_name = excluded.file_name, enabled = excluded.enabled",
            params![
                p.id,
                p.instance_id,
                p.type_kind,
                p.file_name,
                p.enabled as i64,
                p.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_resource_packs(conn: &Connection, instance_id: &str) -> Result<Vec<ResourcePackEntry>, RmclError> {
        let mut stmt = conn.prepare(
            "SELECT id, instance_id, type, file_name, enabled, created_at
             FROM resource_packs WHERE instance_id = ?1 ORDER BY type, file_name",
        )?;
        let rows = stmt.query_map([instance_id], |row| {
            Ok(ResourcePackEntry {
                id: row.get(0)?,
                instance_id: row.get(1)?,
                type_kind: row.get(2)?,
                file_name: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_resource_pack(conn: &Connection, id: &str) -> Result<Option<ResourcePackEntry>, RmclError> {
        let p = conn
            .query_row(
                "SELECT id, instance_id, type, file_name, enabled, created_at
                 FROM resource_packs WHERE id = ?1",
                [id],
                |row| {
                    Ok(ResourcePackEntry {
                        id: row.get(0)?,
                        instance_id: row.get(1)?,
                        type_kind: row.get(2)?,
                        file_name: row.get(3)?,
                        enabled: row.get::<_, i64>(4)? != 0,
                        created_at: row.get(5)?,
                    })
                },
            )
            .optional()?;
        Ok(p)
    }

    pub fn delete_resource_pack(conn: &Connection, id: &str) -> Result<(), RmclError> {
        conn.execute("DELETE FROM resource_packs WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn set_resource_pack_enabled(conn: &Connection, id: &str, enabled: bool) -> Result<(), RmclError> {
        conn.execute("UPDATE resource_packs SET enabled = ?1 WHERE id = ?2", params![enabled as i64, id])?;
        Ok(())
    }
}
