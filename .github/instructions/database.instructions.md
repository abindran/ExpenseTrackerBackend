---
description: "Use when writing SQL migrations, D1 queries, or modifying database schema. Covers D1 SQLite dialect, soft-delete pattern, and migration conventions."
applyTo: "migrations/**/*.sql"
---
# D1 / SQL Conventions

- D1 is SQLite-based — use SQLite syntax (e.g. `strftime`, `TEXT` not `VARCHAR`)
- Timestamps: `TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))` — ISO 8601
- Always include `updated_at` column with default and trigger-like update via app code
- Soft deletes: `soft_deleted INTEGER DEFAULT 0` — never use `DELETE FROM`
- Primary keys: `TEXT PRIMARY KEY` (UUIDv4 strings, not autoincrement)
- Foreign keys: always declare `REFERENCES` with `ON DELETE` behavior
- New migrations go in `migrations/NNNN_description/up.sql` and `down.sql`
- Apply with: `npx wrangler d1 execute <db-name> --file=migrations/<name>/up.sql --remote`
- D1 bind params are positional: `?1`, `?2`, etc.
- Use `COALESCE(?N, column)` for partial updates (PATCH semantics)
