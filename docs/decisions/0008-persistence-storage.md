# ADR 0008：SQLite 持久化边界

- 状态：Accepted；rc.2 修订
- 日期：2026-07-19，修订于 2026-07-24

## 决定

- SQLite 只保存用户播放列表、稠密排序的列表项目和应用状态；默认列表、播放执行序列、最近播放、媒体索引和封面不持久化。
- schema 版本集中管理，迁移在事务内按顺序执行且幂等。schema v5 删除旧 `recent_plays`；更早迁移已删除 `managed_folders` 与 `media_records`。
- 数据访问只经 persistence adapter；播放、React 和 Tauri command 不直接执行 SQL。
- 用户列表写入使用事务并保持项目位置稠密。迁移失败不得部分提交或静默重建数据库。

## 理由与后果

SQLite 适合小型关系数据和可测试迁移，但不是媒体库。收窄表集合可减少冷启动、失效通知和隐私负担。每次 schema 变更必须用仍可能存在的旧版本 fixture 证明用户播放列表、项目顺序和应用状态保持。
