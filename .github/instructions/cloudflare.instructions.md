---
description: "Use when modifying wrangler.toml, adding bindings, configuring environments, or working with Cloudflare KV cache. Covers Workers deployment and cache invalidation patterns."
applyTo: "wrangler.toml"
---
# Cloudflare Workers & Cache

## Environments
- Default config = **dev** (D1: `expense-tracker-db-dev`, KV: dev namespace)
- `[env.production]` = **prod** — deploy with `npx wrangler deploy --env production`
- Secrets (e.g. `CLERK_SECRET_KEY`) are set per-env via `npx wrangler secret put`

## KV Cache Pattern
- `Cache::new(env)` returns `Option<Cache>` — always handle `None` gracefully
- Read-through: check KV → miss → query D1 → store in KV with TTL
- Write-through invalidation: every mutation must `cache.delete()` the relevant key
- Key format: `{entity}:{user_id}:{resource}` (e.g. `user:abc:expenses`)
- KV minimum TTL is 60 seconds
- Cache hits return an `X-Cache: HIT` header

## Bindings
- D1 database: binding name `DB`
- KV namespace: binding name `CACHE`
- Secrets: `CLERK_SECRET_KEY`
- Adding a new binding? Add to both default and `[env.production]` sections
