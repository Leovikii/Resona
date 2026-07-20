# ADR 0008: SQLite persistence boundary

- Status: Accepted
- Date: 2026-07-18

## Decision

Resona uses `rusqlite` with the `bundled` feature as the first persistence adapter. The database is opened under Tauri `app_data_dir()` as `resona.sqlite3`; schema changes are applied through an idempotent `PRAGMA user_version` migration.

Rust owns the connection, migrations, transactions, validation and DTO mapping. Tauri commands expose typed serializable records only. React features call those commands through the shared bridge and never execute SQL or depend on SQLite row types.

## Rationale

This keeps the first local database offline, self-contained and independent of a second frontend database runtime. It also preserves a narrow adapter boundary for a future migration if requirements justify another storage engine.

## Consequences

- The bundled SQLite library adds build size and compile time, but avoids a machine-level SQLite prerequisite.
- Database migration failures are surfaced as typed, diagnosable command failures.
- Playlist and recent-play records remain separate from the in-memory playback queue.
- Any future schema change must add a migration step and regression test; dependency or storage-engine changes require a new ADR.
