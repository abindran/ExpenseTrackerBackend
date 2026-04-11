// Pure business logic — always compiled, testable natively with `cargo test`
pub mod validation;

// WASM-only: Cloudflare Worker runtime, D1 bindings, HTTP routing, and Clerk auth
#[cfg(target_arch = "wasm32")]
mod cache;

#[cfg(target_arch = "wasm32")]
mod models;

#[cfg(target_arch = "wasm32")]
mod routes;

#[cfg(target_arch = "wasm32")]
use worker::*;

#[cfg(target_arch = "wasm32")]
#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        // Health
        .get_async("/health", routes::health)
        // User profile (first-login upsert)
        .post_async("/api/users/me", routes::upsert_user)
        .get_async("/api/users/me", routes::get_me)
        // Expenses
        .get_async("/api/expenses", routes::list_expenses)
        .post_async("/api/expenses", routes::create_expense)
        .get_async("/api/expenses/:id", routes::get_expense)
        .put_async("/api/expenses/:id", routes::update_expense)
        .delete_async("/api/expenses/:id", routes::delete_expense)
        // Categories
        .get_async("/api/categories", routes::list_categories)
        .post_async("/api/categories", routes::create_category)
        .put_async("/api/categories/:id", routes::update_category)
        .delete_async("/api/categories/:id", routes::delete_category)
        // Tags
        .get_async("/api/tags", routes::list_tags)
        .post_async("/api/tags", routes::create_tag)
        .delete_async("/api/tags/:id", routes::delete_tag)
        // Sync
        .post_async("/api/sync", routes::sync)
        // Fallback
        .run(req, env)
        .await
}
