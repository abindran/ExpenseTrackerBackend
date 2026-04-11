# Expense Tracker Backend

REST API for the Expense Tracker app — built with **Rust**, compiled to **WASM**, and deployed on **Cloudflare Workers**.

| Layer | Technology |
|-------|-----------|
| Runtime | Cloudflare Workers (WASM) |
| Database | Cloudflare D1 (SQLite) |
| Cache | Cloudflare KV |
| Auth | Clerk (JWT) |
| Language | Rust 2021 edition |

## Prerequisites

- [Rust](https://rustup.rs/) with the WASM target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- [Node.js](https://nodejs.org/) (v18+) — for Wrangler CLI
- [worker-build](https://crates.io/crates/worker-build):
  ```sh
  cargo install worker-build
  ```
- A [Cloudflare account](https://dash.cloudflare.com/sign-up) with Wrangler authenticated:
  ```sh
  npx wrangler login
  ```
- A [Clerk](https://clerk.com/) account — you'll need a `CLERK_SECRET_KEY`

## Project Structure

```
src/
├── lib.rs          # Entry point, route registration
├── routes.rs       # HTTP handlers, auth, D1 queries
├── models.rs       # Request/response structs (serde)
├── cache.rs        # KV cache wrapper with TTL helpers
└── validation.rs   # Pure business logic (native-testable)
migrations/         # D1 SQL schema migrations
docs/
├── openapi.yaml    # OpenAPI 3.1 spec
└── preview.sh      # Build & open API docs in browser
.github/
├── copilot-instructions.md          # AI assistant context
├── instructions/                    # File-specific AI rules
└── agents/docs-sync.agent.md       # Auto-sync docs agent
```

## Local Development

### 1. Set secrets

Create a `.dev.vars` file in the project root (git-ignored):

```
CLERK_SECRET_KEY=sk_test_your_clerk_secret_key_here
```

### 2. Apply migrations to dev database

```sh
npx wrangler d1 execute expense-tracker-db-dev \
  --file=migrations/0001_initial_schema/up.sql --remote
```

### 3. Start the dev server

```sh
npx wrangler dev
```

This starts a local server at `http://localhost:8787` using the **dev** D1 database and KV namespace.

### 4. Test an endpoint

```sh
curl http://localhost:8787/health
# {"status":"ok","auth":"clerk"}
```

Authenticated endpoints require a `Bearer <clerk-jwt>` header.

## Build & Test

### Build

```sh
# Check WASM compilation without a full build
cargo check --target wasm32-unknown-unknown

# Full WASM build (outputs to build/worker/shim.mjs)
worker-build --release
```

### Unit Tests

The `validation` module compiles natively, so tests run without WASM:

```sh
cargo test
```

This runs 46 tests covering all validation functions (amounts, dates, currencies, names, emojis, descriptions, emails, passwords) including boundary and edge cases.

### Integration Tests

`tests/integration.sh` is a curl-based script that hits a running dev server. Start the server first, then run the tests:

```sh
# Terminal 1 — start the dev server
npx wrangler dev

# Terminal 2 — run integration tests
./tests/integration.sh
```

To test authenticated endpoints, export a valid Clerk JWT before running:

```sh
export CLERK_JWT="your-clerk-jwt-token"
./tests/integration.sh
```

Without `CLERK_JWT`, only unauthenticated tests (health check, 401 responses) will run.

## Deploy

### Development

```sh
npx wrangler deploy
```

Deploys using the default config in `wrangler.toml` (dev D1 + dev KV).

### Production

**First time only** — set the production secret:

```sh
npx wrangler secret put CLERK_SECRET_KEY --env production
```

Apply migrations to the production database:

```sh
npx wrangler d1 execute expense-tracker-db \
  --file=migrations/0001_initial_schema/up.sql --remote
```

Deploy:

```sh
npx wrangler deploy --env production
```

## Database Migrations

Migrations live in `migrations/`. Each migration has an `up.sql` and `down.sql`.

```sh
# Apply a migration to dev
npx wrangler d1 execute expense-tracker-db-dev \
  --file=migrations/<name>/up.sql --remote

# Apply a migration to production
npx wrangler d1 execute expense-tracker-db \
  --file=migrations/<name>/up.sql --remote
```

## API Documentation

The full OpenAPI 3.1 spec is at `docs/openapi.yaml`.

To build and view it as interactive HTML docs:

```sh
./docs/preview.sh
```

This generates `docs/index.html` using [Redocly](https://redocly.com/) and opens it in your browser.

## API Overview

All endpoints (except `/health`) require a Clerk JWT in the `Authorization: Bearer <token>` header. Responses use the envelope format `{ success, data, error }`.

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/api/users/me` | Create/update user profile |
| GET | `/api/users/me` | Get current user |
| GET | `/api/expenses` | List expenses |
| POST | `/api/expenses` | Create expense |
| GET | `/api/expenses/:id` | Get expense |
| PUT | `/api/expenses/:id` | Update expense |
| DELETE | `/api/expenses/:id` | Soft-delete expense |
| GET | `/api/categories` | List categories |
| POST | `/api/categories` | Create category |
| PUT | `/api/categories/:id` | Update category |
| DELETE | `/api/categories/:id` | Soft-delete category |
| GET | `/api/tags` | List tags |
| POST | `/api/tags` | Create tag |
| DELETE | `/api/tags/:id` | Soft-delete tag |
| POST | `/api/sync` | Incremental sync |

## Environments

| Environment | D1 Database | KV Namespace | Deploy Command |
|-------------|-------------|--------------|----------------|
| Development | `expense-tracker-db-dev` | dev CACHE | `npx wrangler deploy` |
| Production | `expense-tracker-db` | prod CACHE | `npx wrangler deploy --env production` |

## License

Private — PaxAutomata
