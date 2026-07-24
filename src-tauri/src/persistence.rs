// SPDX-License-Identifier: GPL-3.0-only

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, ErrorCode, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SCHEMA_VERSION: i64 = 5;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("无法打开本地数据：{0}")]
    Open(String),
    #[error("无法迁移本地数据：{0}")]
    Migration(String),
    #[error("本地数据查询失败：{0}")]
    Query(String),
    #[error("播放列表名称不能为空")]
    EmptyName,
    #[error("播放列表名称已存在")]
    DuplicateName,
    #[error("播放列表不存在")]
    PlaylistNotFound,
    #[error("播放列表项目不存在")]
    PlaylistItemNotFound,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceFailure {
    pub code: String,
    pub message: String,
}

impl PersistenceError {
    pub fn failure(&self) -> PersistenceFailure {
        let code = match self {
            Self::EmptyName => "empty_name",
            Self::DuplicateName => "duplicate_playlist_name",
            Self::PlaylistNotFound => "playlist_not_found",
            Self::PlaylistItemNotFound => "playlist_item_not_found",
            Self::Open(_) => "persistence_open_failed",
            Self::Migration(_) => "persistence_migration_failed",
            Self::Query(_) => "persistence_query_failed",
        };
        PersistenceFailure {
            code: code.to_owned(),
            message: self.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub position: i64,
    pub item_count: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItemRecord {
    pub id: i64,
    pub playlist_id: i64,
    pub path: String,
    pub display_name: String,
    pub position: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetails {
    pub playlist: PlaylistSummary,
    pub items: Vec<PlaylistItemRecord>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSessionRecord {
    pub queue_paths: Vec<String>,
    pub current_path: Option<String>,
    pub position_ms: u64,
    pub volume: f32,
    pub playback_mode: String,
    pub selected_output_device_id: Option<String>,
}

pub struct PersistenceService {
    connection: Mutex<Connection>,
}

impl PersistenceService {
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        let connection =
            Connection::open(path).map_err(|error| PersistenceError::Open(error.to_string()))?;
        configure_connection(&connection)?;
        migrate(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn list_playlists(&self) -> Result<Vec<PlaylistSummary>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        list_playlists_with_connection(&connection)
    }

    pub fn create_playlist_with_items(
        &self,
        name: &str,
        paths: &[String],
        position: Option<i64>,
        ensure_unique_name: bool,
    ) -> Result<PlaylistDetails, PersistenceError> {
        let requested_name = clean_name(name)?;
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        normalize_playlist_positions(&transaction)?;
        let playlist_count = playlist_count(&transaction)?;
        let target_position = position.unwrap_or(playlist_count).clamp(0, playlist_count);
        let name = if ensure_unique_name {
            unique_playlist_name(&transaction, requested_name)?
        } else {
            requested_name.to_owned()
        };

        transaction
            .execute(
                "UPDATE playlists SET position = position + 1 WHERE position >= ?1",
                params![target_position],
            )
            .map_err(query_error)?;
        let now = now_seconds();
        transaction
            .execute(
                "INSERT INTO playlists (name, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)",
                params![name, target_position, now],
            )
            .map_err(name_error)?;
        let playlist_id = transaction.last_insert_rowid();
        insert_playlist_items(&transaction, playlist_id, paths, 0)?;
        transaction.commit().map_err(query_error)?;

        playlist_details_by_id(&connection, playlist_id)
    }

    pub fn rename_playlist(
        &self,
        id: i64,
        name: &str,
    ) -> Result<PlaylistSummary, PersistenceError> {
        let name = clean_name(name)?;
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        let changed = connection
            .execute(
                "UPDATE playlists SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now_seconds(), id],
            )
            .map_err(name_error)?;
        if changed == 0 {
            return Err(PersistenceError::PlaylistNotFound);
        }
        summary_by_id(&connection, id)
    }

    pub fn delete_playlist(&self, id: i64) -> Result<(), PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        let changed = transaction
            .execute("DELETE FROM playlists WHERE id = ?1", params![id])
            .map_err(query_error)?;
        if changed == 0 {
            return Err(PersistenceError::PlaylistNotFound);
        }
        normalize_playlist_positions(&transaction)?;
        transaction.commit().map_err(query_error)
    }

    pub fn move_playlist(
        &self,
        id: i64,
        to_position: i64,
    ) -> Result<Vec<PlaylistSummary>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        normalize_playlist_positions(&transaction)?;
        let current_position = transaction
            .query_row(
                "SELECT position FROM playlists WHERE id = ?1",
                params![id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(query_error)?
            .ok_or(PersistenceError::PlaylistNotFound)?;
        let count = playlist_count(&transaction)?;
        let target_position = to_position.clamp(0, count.saturating_sub(1));

        if current_position < target_position {
            transaction
                .execute(
                    "UPDATE playlists SET position = position - 1
                     WHERE position > ?1 AND position <= ?2",
                    params![current_position, target_position],
                )
                .map_err(query_error)?;
        } else if current_position > target_position {
            transaction
                .execute(
                    "UPDATE playlists SET position = position + 1
                     WHERE position >= ?1 AND position < ?2",
                    params![target_position, current_position],
                )
                .map_err(query_error)?;
        }
        transaction
            .execute(
                "UPDATE playlists SET position = ?1 WHERE id = ?2",
                params![target_position, id],
            )
            .map_err(query_error)?;
        transaction.commit().map_err(query_error)?;
        list_playlists_with_connection(&connection)
    }

    pub fn list_playlist_items(
        &self,
        playlist_id: i64,
    ) -> Result<Vec<PlaylistItemRecord>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        ensure_playlist_exists(&connection, playlist_id)?;
        list_playlist_items_with_connection(&connection, playlist_id)
    }

    pub fn add_playlist_items(
        &self,
        playlist_id: i64,
        paths: &[String],
        position: Option<i64>,
    ) -> Result<Vec<PlaylistItemRecord>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        ensure_playlist_exists(&connection, playlist_id)?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        normalize_item_positions(&transaction, playlist_id)?;
        let item_count = playlist_item_count(&transaction, playlist_id)?;
        let start_position = position.unwrap_or(item_count).clamp(0, item_count);
        insert_playlist_items(&transaction, playlist_id, paths, start_position)?;
        if !paths.is_empty() {
            touch_playlist(&transaction, playlist_id)?;
        }
        transaction.commit().map_err(query_error)?;
        list_playlist_items_with_connection(&connection, playlist_id)
    }

    pub fn remove_playlist_item(
        &self,
        playlist_id: i64,
        item_id: i64,
    ) -> Result<Vec<PlaylistItemRecord>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        ensure_playlist_exists(&connection, playlist_id)?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        let changed = transaction
            .execute(
                "DELETE FROM playlist_items WHERE playlist_id = ?1 AND id = ?2",
                params![playlist_id, item_id],
            )
            .map_err(query_error)?;
        if changed == 0 {
            return Err(PersistenceError::PlaylistItemNotFound);
        }
        normalize_item_positions(&transaction, playlist_id)?;
        touch_playlist(&transaction, playlist_id)?;
        transaction.commit().map_err(query_error)?;
        list_playlist_items_with_connection(&connection, playlist_id)
    }

    pub fn remove_playlist_items(
        &self,
        playlist_id: i64,
        item_ids: &[i64],
    ) -> Result<Vec<PlaylistItemRecord>, PersistenceError> {
        if item_ids.is_empty() {
            return self.list_playlist_items(playlist_id);
        }
        let mut unique_ids = item_ids.to_vec();
        unique_ids.sort_unstable();
        unique_ids.dedup();
        if unique_ids.len() != item_ids.len() {
            return Err(PersistenceError::PlaylistItemNotFound);
        }
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        ensure_playlist_exists(&connection, playlist_id)?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        let existing = list_playlist_items_with_connection(&transaction, playlist_id)?;
        let existing_ids = existing
            .iter()
            .map(|item| item.id)
            .collect::<std::collections::HashSet<_>>();
        if !unique_ids
            .iter()
            .all(|item_id| existing_ids.contains(item_id))
        {
            return Err(PersistenceError::PlaylistItemNotFound);
        }
        for item_id in unique_ids {
            transaction
                .execute(
                    "DELETE FROM playlist_items WHERE playlist_id = ?1 AND id = ?2",
                    params![playlist_id, item_id],
                )
                .map_err(query_error)?;
        }
        normalize_item_positions(&transaction, playlist_id)?;
        touch_playlist(&transaction, playlist_id)?;
        transaction.commit().map_err(query_error)?;
        list_playlist_items_with_connection(&connection, playlist_id)
    }

    pub fn clear_playlist_items(
        &self,
        playlist_id: i64,
    ) -> Result<Vec<PlaylistItemRecord>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        ensure_playlist_exists(&connection, playlist_id)?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        transaction
            .execute(
                "DELETE FROM playlist_items WHERE playlist_id = ?1",
                params![playlist_id],
            )
            .map_err(query_error)?;
        touch_playlist(&transaction, playlist_id)?;
        transaction.commit().map_err(query_error)?;
        Ok(Vec::new())
    }

    pub fn move_playlist_item(
        &self,
        playlist_id: i64,
        item_id: i64,
        to_position: i64,
    ) -> Result<Vec<PlaylistItemRecord>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        ensure_playlist_exists(&connection, playlist_id)?;
        let transaction = connection.unchecked_transaction().map_err(query_error)?;
        normalize_item_positions(&transaction, playlist_id)?;
        let current_position = transaction
            .query_row(
                "SELECT position FROM playlist_items WHERE playlist_id = ?1 AND id = ?2",
                params![playlist_id, item_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(query_error)?
            .ok_or(PersistenceError::PlaylistItemNotFound)?;
        let item_count = playlist_item_count(&transaction, playlist_id)?;
        let target_position = to_position.clamp(0, item_count.saturating_sub(1));

        if current_position < target_position {
            transaction
                .execute(
                    "UPDATE playlist_items SET position = position - 1
                     WHERE playlist_id = ?1 AND position > ?2 AND position <= ?3",
                    params![playlist_id, current_position, target_position],
                )
                .map_err(query_error)?;
        } else if current_position > target_position {
            transaction
                .execute(
                    "UPDATE playlist_items SET position = position + 1
                     WHERE playlist_id = ?1 AND position >= ?2 AND position < ?3",
                    params![playlist_id, target_position, current_position],
                )
                .map_err(query_error)?;
        }
        transaction
            .execute(
                "UPDATE playlist_items SET position = ?1 WHERE playlist_id = ?2 AND id = ?3",
                params![target_position, playlist_id, item_id],
            )
            .map_err(query_error)?;
        touch_playlist(&transaction, playlist_id)?;
        transaction.commit().map_err(query_error)?;
        list_playlist_items_with_connection(&connection, playlist_id)
    }

    pub fn load_playback_session(&self) -> Result<Option<PlaybackSessionRecord>, PersistenceError> {
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        let value = connection
            .query_row(
                "SELECT value FROM app_state WHERE key = 'playback_session'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(query_error)?;
        value
            .map(|value| serde_json::from_str(&value).map_err(json_error))
            .transpose()
    }

    pub fn save_playback_session(
        &self,
        session: &PlaybackSessionRecord,
    ) -> Result<(), PersistenceError> {
        let value = serde_json::to_string(session).map_err(json_error)?;
        let connection = self.connection.lock().map_err(|_| poisoned())?;
        connection
            .execute(
                "INSERT INTO app_state (key, value) VALUES ('playback_session', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![value],
            )
            .map_err(query_error)?;
        Ok(())
    }
}

fn configure_connection(connection: &Connection) -> Result<(), PersistenceError> {
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| PersistenceError::Open(error.to_string()))
}

fn migrate(connection: &Connection) -> Result<(), PersistenceError> {
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| PersistenceError::Migration(error.to_string()))?;
    if version > SCHEMA_VERSION {
        return Err(PersistenceError::Migration(format!(
            "数据库版本 {version} 高于应用支持的版本 {SCHEMA_VERSION}"
        )));
    }
    if version == 0 {
        connection
            .execute_batch(
                "BEGIN;
                 CREATE TABLE playlists (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   name TEXT NOT NULL UNIQUE,
                   position INTEGER NOT NULL,
                   created_at INTEGER NOT NULL,
                   updated_at INTEGER NOT NULL
                 );
                 CREATE INDEX idx_playlists_order ON playlists(position, id);
                 CREATE TABLE playlist_items (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                   path TEXT NOT NULL,
                   display_name TEXT NOT NULL,
                   position INTEGER NOT NULL
                 );
                 CREATE INDEX idx_playlist_items_order
                   ON playlist_items(playlist_id, position, id);
                 CREATE TABLE app_state (
                   key TEXT PRIMARY KEY,
                   value TEXT NOT NULL
                 );
                 PRAGMA user_version = 5;
                 COMMIT;",
            )
            .map_err(|error| PersistenceError::Migration(error.to_string()))?;
        return Ok(());
    }
    if version < 3 {
        migrate_legacy_schema(connection)?;
    }
    if version < 4 {
        connection
            .execute_batch(
                "BEGIN;
                 CREATE TABLE IF NOT EXISTS app_state (
                   key TEXT PRIMARY KEY,
                   value TEXT NOT NULL
                 );
                 PRAGMA user_version = 4;
                 COMMIT;",
            )
            .map_err(|error| PersistenceError::Migration(error.to_string()))?;
    }
    if version < 5 {
        connection
            .execute_batch(
                "BEGIN;
                 DROP TABLE IF EXISTS recent_plays;
                 PRAGMA user_version = 5;
                 COMMIT;",
            )
            .map_err(|error| PersistenceError::Migration(error.to_string()))?;
    }
    Ok(())
}

fn migrate_legacy_schema(connection: &Connection) -> Result<(), PersistenceError> {
    let transaction = connection
        .unchecked_transaction()
        .map_err(migration_error)?;
    transaction
        .execute(
            "ALTER TABLE playlists ADD COLUMN position INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(migration_error)?;
    let playlist_ids = {
        let mut statement = transaction
            .prepare("SELECT id FROM playlists ORDER BY updated_at DESC, id DESC")
            .map_err(migration_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(migration_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(migration_error)?;
        rows
    };
    for (position, id) in playlist_ids.into_iter().enumerate() {
        transaction
            .execute(
                "UPDATE playlists SET position = ?1 WHERE id = ?2",
                params![position as i64, id],
            )
            .map_err(migration_error)?;
    }
    transaction
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_playlists_order ON playlists(position, id);
             DROP TABLE IF EXISTS media_records;
             DROP TABLE IF EXISTS managed_folders;
             PRAGMA user_version = 3;",
        )
        .map_err(migration_error)?;
    transaction.commit().map_err(migration_error)
}

fn list_playlists_with_connection(
    connection: &Connection,
) -> Result<Vec<PlaylistSummary>, PersistenceError> {
    let mut statement = connection
        .prepare(
            "SELECT p.id, p.name, p.position, p.created_at, p.updated_at, COUNT(i.id)
             FROM playlists p
             LEFT JOIN playlist_items i ON i.playlist_id = p.id
             GROUP BY p.id
             ORDER BY p.position, p.id",
        )
        .map_err(query_error)?;
    let rows = statement
        .query_map([], playlist_summary_from_row)
        .map_err(query_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
}

fn summary_by_id(connection: &Connection, id: i64) -> Result<PlaylistSummary, PersistenceError> {
    connection
        .query_row(
            "SELECT p.id, p.name, p.position, p.created_at, p.updated_at, COUNT(i.id)
             FROM playlists p
             LEFT JOIN playlist_items i ON i.playlist_id = p.id
             WHERE p.id = ?1
             GROUP BY p.id",
            params![id],
            playlist_summary_from_row,
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => PersistenceError::PlaylistNotFound,
            other => query_error(other),
        })
}

fn playlist_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlaylistSummary> {
    Ok(PlaylistSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        position: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        item_count: row.get::<_, i64>(5)?.max(0) as u64,
    })
}

fn playlist_details_by_id(
    connection: &Connection,
    id: i64,
) -> Result<PlaylistDetails, PersistenceError> {
    Ok(PlaylistDetails {
        playlist: summary_by_id(connection, id)?,
        items: list_playlist_items_with_connection(connection, id)?,
    })
}

fn list_playlist_items_with_connection(
    connection: &Connection,
    playlist_id: i64,
) -> Result<Vec<PlaylistItemRecord>, PersistenceError> {
    let mut statement = connection
        .prepare(
            "SELECT id, playlist_id, path, display_name, position
             FROM playlist_items
             WHERE playlist_id = ?1
             ORDER BY position, id",
        )
        .map_err(query_error)?;
    let rows = statement
        .query_map(params![playlist_id], |row| {
            Ok(PlaylistItemRecord {
                id: row.get(0)?,
                playlist_id: row.get(1)?,
                path: row.get(2)?,
                display_name: row.get(3)?,
                position: row.get(4)?,
            })
        })
        .map_err(query_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
}

fn insert_playlist_items(
    transaction: &Transaction<'_>,
    playlist_id: i64,
    paths: &[String],
    start_position: i64,
) -> Result<(), PersistenceError> {
    if paths.is_empty() {
        return Ok(());
    }
    transaction
        .execute(
            "UPDATE playlist_items SET position = position + ?1
             WHERE playlist_id = ?2 AND position >= ?3",
            params![paths.len() as i64, playlist_id, start_position],
        )
        .map_err(query_error)?;
    for (offset, path) in paths.iter().enumerate() {
        let display_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path);
        transaction
            .execute(
                "INSERT INTO playlist_items
                 (playlist_id, path, display_name, position)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    playlist_id,
                    path,
                    display_name,
                    start_position + offset as i64
                ],
            )
            .map_err(query_error)?;
    }
    Ok(())
}

fn ensure_playlist_exists(
    connection: &Connection,
    playlist_id: i64,
) -> Result<(), PersistenceError> {
    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM playlists WHERE id = ?1)",
            params![playlist_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(query_error)?;
    if exists {
        Ok(())
    } else {
        Err(PersistenceError::PlaylistNotFound)
    }
}

fn playlist_count(connection: &Connection) -> Result<i64, PersistenceError> {
    connection
        .query_row("SELECT COUNT(*) FROM playlists", [], |row| row.get(0))
        .map_err(query_error)
}

fn playlist_item_count(connection: &Connection, playlist_id: i64) -> Result<i64, PersistenceError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM playlist_items WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get(0),
        )
        .map_err(query_error)
}

fn normalize_playlist_positions(connection: &Connection) -> Result<(), PersistenceError> {
    let ids = {
        let mut statement = connection
            .prepare("SELECT id FROM playlists ORDER BY position, id")
            .map_err(query_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(query_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(query_error)?;
        rows
    };
    for (position, id) in ids.into_iter().enumerate() {
        connection
            .execute(
                "UPDATE playlists SET position = ?1 WHERE id = ?2",
                params![position as i64, id],
            )
            .map_err(query_error)?;
    }
    Ok(())
}

fn normalize_item_positions(
    connection: &Connection,
    playlist_id: i64,
) -> Result<(), PersistenceError> {
    let ids = {
        let mut statement = connection
            .prepare(
                "SELECT id FROM playlist_items
                 WHERE playlist_id = ?1 ORDER BY position, id",
            )
            .map_err(query_error)?;
        let rows = statement
            .query_map(params![playlist_id], |row| row.get::<_, i64>(0))
            .map_err(query_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(query_error)?;
        rows
    };
    for (position, id) in ids.into_iter().enumerate() {
        connection
            .execute(
                "UPDATE playlist_items SET position = ?1 WHERE id = ?2",
                params![position as i64, id],
            )
            .map_err(query_error)?;
    }
    Ok(())
}

fn touch_playlist(connection: &Connection, playlist_id: i64) -> Result<(), PersistenceError> {
    connection
        .execute(
            "UPDATE playlists SET updated_at = ?1 WHERE id = ?2",
            params![now_seconds(), playlist_id],
        )
        .map_err(query_error)?;
    Ok(())
}

fn unique_playlist_name(
    connection: &Connection,
    requested_name: &str,
) -> Result<String, PersistenceError> {
    for suffix in 1..=10_000 {
        let candidate = if suffix == 1 {
            requested_name.to_owned()
        } else {
            format!("{requested_name} {suffix}")
        };
        let exists = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM playlists WHERE name = ?1)",
                params![candidate],
                |row| row.get::<_, bool>(0),
            )
            .map_err(query_error)?;
        if !exists {
            return Ok(candidate);
        }
    }
    Err(PersistenceError::Query(
        "无法生成不重复的播放列表名称".to_owned(),
    ))
}

fn clean_name(name: &str) -> Result<&str, PersistenceError> {
    let name = name.trim();
    if name.is_empty() {
        Err(PersistenceError::EmptyName)
    } else {
        Ok(name)
    }
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn name_error(error: rusqlite::Error) -> PersistenceError {
    match &error {
        rusqlite::Error::SqliteFailure(details, _)
            if details.code == ErrorCode::ConstraintViolation =>
        {
            PersistenceError::DuplicateName
        }
        _ => query_error(error),
    }
}

fn query_error(error: rusqlite::Error) -> PersistenceError {
    PersistenceError::Query(error.to_string())
}

fn json_error(error: serde_json::Error) -> PersistenceError {
    PersistenceError::Query(error.to_string())
}

fn migration_error(error: rusqlite::Error) -> PersistenceError {
    PersistenceError::Migration(error.to_string())
}

fn poisoned() -> PersistenceError {
    PersistenceError::Query("本地数据锁已损坏".to_owned())
}

#[cfg(test)]
pub(crate) fn migrate_for_test(connection: &Connection) {
    configure_connection(connection).expect("configure test database");
    migrate(connection).expect("migrate test database");
}

#[cfg(test)]
pub(crate) fn in_memory_for_test() -> PersistenceService {
    let connection = Connection::open_in_memory().expect("open in-memory database");
    migrate_for_test(&connection);
    PersistenceService {
        connection: Mutex::new(connection),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> PersistenceService {
        in_memory_for_test()
    }

    #[test]
    fn migrates_v2_without_losing_playlists() {
        let connection = Connection::open_in_memory().expect("open legacy database");
        connection
            .execute_batch(
                "CREATE TABLE playlists (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   name TEXT NOT NULL UNIQUE,
                   created_at INTEGER NOT NULL,
                   updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE playlist_items (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                   path TEXT NOT NULL,
                   display_name TEXT NOT NULL,
                   position INTEGER NOT NULL
                 );
                 CREATE TABLE recent_plays (
                   path TEXT PRIMARY KEY,
                   display_name TEXT NOT NULL,
                   last_played_at INTEGER NOT NULL,
                   play_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE managed_folders (
                   id INTEGER PRIMARY KEY,
                   path TEXT NOT NULL UNIQUE,
                   last_scan_at INTEGER,
                   scan_status TEXT NOT NULL
                 );
                 CREATE TABLE media_records (path TEXT PRIMARY KEY);
                 INSERT INTO playlists (id, name, created_at, updated_at)
                   VALUES (1, 'Older', 1, 10), (2, 'Newer', 2, 20);
                 INSERT INTO playlist_items
                   (playlist_id, path, display_name, position)
                   VALUES (1, 'C:\\Music\\one.wav', 'one.wav', 0);
                 INSERT INTO recent_plays
                   (path, display_name, last_played_at, play_count)
                   VALUES ('C:\\Music\\one.wav', 'one.wav', 30, 2);
                 PRAGMA user_version = 2;",
            )
            .expect("create legacy schema");

        migrate_for_test(&connection);

        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, 5);
        let service = PersistenceService {
            connection: Mutex::new(connection),
        };
        let playlists = service.list_playlists().expect("list migrated playlists");
        assert_eq!(
            playlists
                .iter()
                .map(|playlist| (playlist.name.as_str(), playlist.position))
                .collect::<Vec<_>>(),
            vec![("Newer", 0), ("Older", 1)]
        );
        assert_eq!(
            service
                .list_playlist_items(1)
                .expect("list migrated items")
                .len(),
            1
        );
        let connection = service.connection.lock().expect("lock database");
        assert!(!table_exists(&connection, "managed_folders"));
        assert!(!table_exists(&connection, "media_records"));
        assert!(!table_exists(&connection, "recent_plays"));
    }

    #[test]
    fn migrates_v4_by_dropping_only_recent_history() {
        let connection = Connection::open_in_memory().expect("open v4 database");
        connection
            .execute_batch(
                "CREATE TABLE playlists (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   name TEXT NOT NULL UNIQUE,
                   position INTEGER NOT NULL,
                   created_at INTEGER NOT NULL,
                   updated_at INTEGER NOT NULL
                 );
                 CREATE INDEX idx_playlists_order ON playlists(position, id);
                 CREATE TABLE playlist_items (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                   path TEXT NOT NULL,
                   display_name TEXT NOT NULL,
                   position INTEGER NOT NULL
                 );
                 CREATE INDEX idx_playlist_items_order
                   ON playlist_items(playlist_id, position, id);
                 CREATE TABLE recent_plays (
                   path TEXT PRIMARY KEY,
                   display_name TEXT NOT NULL,
                   last_played_at INTEGER NOT NULL,
                   play_count INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE app_state (
                   key TEXT PRIMARY KEY,
                   value TEXT NOT NULL
                 );
                 INSERT INTO playlists (id, name, position, created_at, updated_at)
                   VALUES (1, 'Keep', 0, 1, 2);
                 INSERT INTO playlist_items (id, playlist_id, path, display_name, position)
                   VALUES (1, 1, 'C:\\Music\\keep.wav', 'keep.wav', 0);
                 INSERT INTO recent_plays (path, display_name, last_played_at, play_count)
                   VALUES ('C:\\Music\\drop.wav', 'drop.wav', 3, 1);
                 INSERT INTO app_state (key, value) VALUES ('keep', 'value');
                 PRAGMA user_version = 4;",
            )
            .expect("create v4 schema");

        migrate_for_test(&connection);

        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, 5);
        assert!(!table_exists(&connection, "recent_plays"));
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM playlists", [], |row| row
                    .get::<_, i64>(0))
                .expect("playlist count"),
            1
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM playlist_items", [], |row| row
                    .get::<_, i64>(0))
                .expect("playlist item count"),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT value FROM app_state WHERE key = 'keep'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .expect("app state"),
            "value"
        );
    }

    #[test]
    fn migrates_v3_by_adding_app_state_and_preserving_playlists() {
        let connection = Connection::open_in_memory().expect("open v3 database");
        connection
            .execute_batch(
                "CREATE TABLE playlists (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   name TEXT NOT NULL UNIQUE,
                   position INTEGER NOT NULL,
                   created_at INTEGER NOT NULL,
                   updated_at INTEGER NOT NULL
                 );
                 CREATE INDEX idx_playlists_order ON playlists(position, id);
                 CREATE TABLE playlist_items (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                   path TEXT NOT NULL,
                   display_name TEXT NOT NULL,
                   position INTEGER NOT NULL
                 );
                 CREATE INDEX idx_playlist_items_order
                   ON playlist_items(playlist_id, position, id);
                 CREATE TABLE recent_plays (
                   path TEXT PRIMARY KEY,
                   display_name TEXT NOT NULL,
                   last_played_at INTEGER NOT NULL,
                   play_count INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO playlists (id, name, position, created_at, updated_at)
                   VALUES (1, 'Keep v3', 0, 1, 2);
                 INSERT INTO playlist_items (id, playlist_id, path, display_name, position)
                   VALUES (1, 1, 'C:\\Music\\v3.wav', 'v3.wav', 0);
                 INSERT INTO recent_plays (path, display_name, last_played_at, play_count)
                   VALUES ('C:\\Music\\drop.wav', 'drop.wav', 3, 1);
                 PRAGMA user_version = 3;",
            )
            .expect("create v3 schema");

        migrate_for_test(&connection);

        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("read schema version");
        assert_eq!(version, 5);
        assert!(table_exists(&connection, "app_state"));
        assert!(!table_exists(&connection, "recent_plays"));
        assert_eq!(
            connection
                .query_row("SELECT name FROM playlists WHERE id = 1", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .expect("playlist name"),
            "Keep v3"
        );
        assert_eq!(
            connection
                .query_row("SELECT path FROM playlist_items WHERE id = 1", [], |row| {
                    row.get::<_, String>(0)
                })
                .expect("playlist item path"),
            "C:\\Music\\v3.wav"
        );
    }

    #[test]
    fn creates_deletes_and_moves_playlists_with_dense_positions() {
        let service = service();
        let first = service
            .create_playlist_with_items("First", &[], None, false)
            .expect("create first");
        let second = service
            .create_playlist_with_items("Second", &[], None, false)
            .expect("create second");
        let middle = service
            .create_playlist_with_items("Middle", &[], Some(1), false)
            .expect("create middle");
        assert_eq!(middle.playlist.position, 1);
        assert_eq!(
            service
                .list_playlists()
                .expect("list playlists")
                .iter()
                .map(|playlist| (playlist.name.as_str(), playlist.position))
                .collect::<Vec<_>>(),
            vec![("First", 0), ("Middle", 1), ("Second", 2)]
        );

        service
            .move_playlist(second.playlist.id, 0)
            .expect("move playlist");
        service
            .delete_playlist(middle.playlist.id)
            .expect("delete playlist");
        assert_eq!(
            service
                .list_playlists()
                .expect("list playlists")
                .iter()
                .map(|playlist| (playlist.id, playlist.position))
                .collect::<Vec<_>>(),
            vec![(second.playlist.id, 0), (first.playlist.id, 1)]
        );
    }

    #[test]
    fn playback_session_round_trips_without_starting_playback() {
        let service = service();
        let session = PlaybackSessionRecord {
            queue_paths: vec!["C:\\Music\\one.wav".to_owned()],
            current_path: Some("C:\\Music\\one.wav".to_owned()),
            position_ms: 12_345,
            volume: 0.42,
            playback_mode: "repeat_all".to_owned(),
            selected_output_device_id: Some("device-1".to_owned()),
        };
        service
            .save_playback_session(&session)
            .expect("save playback session");
        assert_eq!(
            service
                .load_playback_session()
                .expect("load playback session"),
            Some(session)
        );
    }

    #[test]
    fn creates_playlist_and_items_atomically_and_uniquifies_automatic_names() {
        let service = service();
        let paths = vec!["C:\\Music\\First.wav".to_owned()];
        let first = service
            .create_playlist_with_items("Music", &paths, None, true)
            .expect("create first list");
        let second = service
            .create_playlist_with_items("Music", &paths, None, true)
            .expect("create unique list");
        assert_eq!(first.playlist.name, "Music");
        assert_eq!(second.playlist.name, "Music 2");
        assert_eq!(first.items.len(), 1);

        let duplicate = service.create_playlist_with_items("Music", &paths, None, false);
        assert!(matches!(duplicate, Err(PersistenceError::DuplicateName)));
        assert_eq!(service.list_playlists().expect("list playlists").len(), 2);
    }

    #[test]
    fn removes_and_reorders_playlist_items_with_dense_positions() {
        let service = service();
        let playlist = service
            .create_playlist_with_items(
                "Road trip",
                &[
                    "C:\\Music\\First.wav".to_owned(),
                    "C:\\Music\\Second.flac".to_owned(),
                    "C:\\Music\\Third.mp3".to_owned(),
                ],
                None,
                false,
            )
            .expect("create playlist");
        let moved = service
            .move_playlist_item(playlist.playlist.id, playlist.items[2].id, 0)
            .expect("move playlist item");
        assert_eq!(
            moved
                .iter()
                .map(|item| (item.display_name.as_str(), item.position))
                .collect::<Vec<_>>(),
            vec![("Third.mp3", 0), ("First.wav", 1), ("Second.flac", 2)]
        );
        let remaining = service
            .remove_playlist_item(playlist.playlist.id, playlist.items[0].id)
            .expect("remove playlist item");
        assert_eq!(
            remaining
                .iter()
                .map(|item| (item.display_name.as_str(), item.position))
                .collect::<Vec<_>>(),
            vec![("Third.mp3", 0), ("Second.flac", 1)]
        );
    }

    #[test]
    fn removes_many_or_clears_playlist_items_atomically() {
        let service = service();
        let created = service
            .create_playlist_with_items(
                "Selection",
                &[
                    "C:\\Music\\one.flac".to_owned(),
                    "C:\\Music\\two.flac".to_owned(),
                    "C:\\Music\\three.flac".to_owned(),
                ],
                None,
                false,
            )
            .expect("create playlist");
        let retained = service
            .remove_playlist_items(
                created.playlist.id,
                &[created.items[0].id, created.items[2].id],
            )
            .expect("remove selected items");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].path, "C:\\Music\\two.flac");
        assert_eq!(retained[0].position, 0);
        assert!(matches!(
            service.remove_playlist_items(created.playlist.id, &[999_999]),
            Err(PersistenceError::PlaylistItemNotFound)
        ));
        assert_eq!(
            service
                .list_playlist_items(created.playlist.id)
                .expect("items retained after failed transaction")
                .len(),
            1
        );
        assert!(service
            .clear_playlist_items(created.playlist.id)
            .expect("clear playlist")
            .is_empty());
    }

    #[test]
    fn playlist_item_operations_reject_unknown_ids() {
        let service = service();
        assert!(matches!(
            service.list_playlist_items(404),
            Err(PersistenceError::PlaylistNotFound)
        ));
        let playlist = service
            .create_playlist_with_items("Known", &[], None, false)
            .expect("create playlist");
        assert!(matches!(
            service.remove_playlist_item(playlist.playlist.id, 404),
            Err(PersistenceError::PlaylistItemNotFound)
        ));
        assert!(matches!(
            service.move_playlist_item(playlist.playlist.id, 404, 0),
            Err(PersistenceError::PlaylistItemNotFound)
        ));
    }

    fn table_exists(connection: &Connection, table: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                params![table],
                |row| row.get(0),
            )
            .expect("query table existence")
    }
}
