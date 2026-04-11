// ── Cloudflare KV-backed cache (Redis-like layer) ──────────────────────────
//
// Cloudflare KV is a globally distributed key-value store with TTL support.
// This module wraps it with typed helpers so route handlers can do:
//
//   cache.get(&key)              → Option<String>
//   cache.put(&key, &val, ttl)   → ()
//   cache.delete(&key)           → ()
//
// If the KV binding is absent (e.g. local dev without it), Cache::new()
// returns None and the app degrades gracefully to uncached DB queries.

use worker::kv::KvStore;
use worker::Env;

const BINDING: &str = "CACHE";

/// TTL constants in seconds (KV minimum is 60 s).
pub mod ttl {
    pub const SESSION: u64 = 120;     // Clerk session verification
    pub const PROFILE: u64 = 300;     // User profile (5 min)
    pub const EXPENSES: u64 = 60;     // Expense list (volatile)
    pub const CATEGORIES: u64 = 300;  // Category list (5 min)
    pub const TAGS: u64 = 300;        // Tag list (5 min)
}

/// Thin wrapper around Cloudflare KV.
/// All operations silently swallow errors so the app never breaks due to cache.
pub struct Cache(KvStore);

impl Cache {
    /// Returns `Some(Cache)` if the KV binding exists, `None` otherwise.
    pub fn new(env: &Env) -> Option<Self> {
        env.kv(BINDING).ok().map(Self)
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).text().await.ok().flatten()
    }

    pub async fn put(&self, key: &str, value: &str, ttl_secs: u64) {
        if let Ok(builder) = self.0.put(key, value) {
            let _ = builder.expiration_ttl(ttl_secs).execute().await;
        }
    }

    pub async fn delete(&self, key: &str) {
        let _ = self.0.delete(key).await;
    }
}

// ── Key builders ───────────────────────────────────────────────────────────

pub fn session_key(session_id: &str) -> String {
    format!("session:{session_id}")
}

pub fn profile_key(user_id: &str) -> String {
    format!("user:{user_id}:profile")
}

pub fn expenses_key(user_id: &str) -> String {
    format!("user:{user_id}:expenses")
}

pub fn categories_key(user_id: &str) -> String {
    format!("user:{user_id}:categories")
}

pub fn tags_key(user_id: &str) -> String {
    format!("user:{user_id}:tags")
}
