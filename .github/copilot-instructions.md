# Expense Tracker Backend

Rust API running on Cloudflare Workers, compiled to WASM via `worker-build`.

## Architecture

- **Runtime**: Cloudflare Workers (WASM) — stateless, no filesystem, no threads
- **Database**: Cloudflare D1 (SQLite-based), bound as `DB` in wrangler.toml
- **Cache**: Cloudflare KV, bound as `CACHE` — `src/cache.rs` wraps it with TTL helpers
- **Auth**: Clerk JWT — base64-decoded, then verified via Clerk Backend API (cached in KV)
- **Entry point**: `src/lib.rs` → routes registered in `#[event(fetch)]` handler, CORS applied centrally

## Module Layout

| Module | Purpose | WASM-only? |
|--------|---------|------------|
| `lib.rs` | Entry, route registration | Yes (the `#[event]` block) |
| `routes.rs` | HTTP handlers, auth, D1 queries | Yes |
| `models.rs` | Serde structs (request/response types) | Yes |
| `cache.rs` | KV wrapper with TTL, key builders | Yes |
| `validation.rs` | Pure business logic, validates input | **No** — runs native tests |

## Build & Test

```sh
cargo test                                     # Native tests (validation module)
cargo check --target wasm32-unknown-unknown     # WASM compile check
worker-build --release                          # Full WASM build → build/worker/shim.mjs
npx wrangler dev                               # Local dev server (uses dev D1 + KV)
npx wrangler deploy                            # Deploy dev
npx wrangler deploy --env production            # Deploy production
```

## Conventions

- **Amounts are in cents** (i64) — never floating point
- **Soft deletes** — `soft_deleted` column, never `DELETE FROM`
- **All tables have `updated_at`** — used by `/api/sync` for incremental sync
- **`#[cfg(target_arch = "wasm32")]`** gates modules that need Worker APIs — keeps `cargo test` working natively
- **Cache is optional** — `Cache::new()` returns `None` if KV binding is absent; all handlers degrade gracefully
- **Mutations invalidate cache** — every create/update/delete handler calls `cache.delete()` on the relevant key
- **Errors use `ApiResponse<T>`** — `{ success: bool, data: Option<T>, error: Option<String> }`
- **IDs are UUIDv4** — generated server-side via `uuid::Uuid::new_v4()`
- **CORS is centralized in `lib.rs`** — `ALLOWED_ORIGIN` env var; preflight (OPTIONS) handled before routing; header injected on every response

## Database

Schema lives in `migrations/0001_initial_schema/up.sql`. Tables: `users`, `categories`, `expenses`, `tags`, `expense_tags`.

Apply migrations:
```sh
npx wrangler d1 execute expense-tracker-db-dev --file=migrations/<name>/up.sql --remote
```

## Environments (wrangler.toml)

- **Default** = development (dev D1 + dev KV)
- **`[env.production]`** = production (prod D1 + prod KV)

## Dependencies

`worker` 0.8 (d1 feature), `serde`, `serde_json`, `chrono`, `uuid` (v4+js), `base64`. No other crates — keep it minimal.

## Scripts

- `scripts/get-token.sh` — fetches a Clerk JWT from `.dev.vars` for local API testing
