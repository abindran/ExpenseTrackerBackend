use serde_json::json;
use worker::wasm_bindgen::JsValue;
use worker::*;

use crate::models::*;
use crate::validation;

// ── Helpers ────────────────────────────────────────────────────────────────

fn json_response<T: serde::Serialize>(data: &T, status: u16) -> Result<Response> {
    let body = serde_json::to_string(data).map_err(|e| Error::RustError(e.to_string()))?;
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    Ok(Response::ok(body)?.with_headers(headers).with_status(status))
}

fn error_response(msg: &str, status: u16) -> Result<Response> {
    json_response(&ApiResponse::<()>::err(msg.to_string()), status)
}

// ── Clerk JWT verification ─────────────────────────────────────────────────
//
// Strategy (MVP):
//   1. Base64-decode the JWT payload to read `sub` (Clerk user_id) and `exp`.
//   2. Reject tokens whose `exp` is in the past.
//   3. Verify the session is still active by calling Clerk's Backend API.
//      This single HTTP call replaces all crypto crates and eliminates our
//      own credential storage.
//
// TODO(production): Cache JWKS from Clerk, verify RS256 signature locally,
//   and cache valid sessions in Cloudflare KV to avoid per-request API calls.

async fn verify_clerk_token(token: &str, clerk_secret: &str) -> Option<String> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    // 1. Decode JWT payload (middle segment)
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_bytes = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

    // 2. Check expiry using JS Date (WASM has no std time)
    let exp = payload["exp"].as_i64()?;
    let now_ms = js_sys::Date::now() as i64;
    if exp * 1000 < now_ms {
        return None; // Token expired
    }

    // 3. Extract user_id and session_id
    let user_id = payload["sub"].as_str()?.to_string();
    let session_id = payload["sid"].as_str().unwrap_or(&user_id).to_string();

    // 4. Confirm session is active via Clerk Backend API
    let url = format!("https://api.clerk.com/v1/sessions/{}", session_id);
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let mut headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", clerk_secret)).ok()?;
    init.with_headers(headers);

    let req = Request::new_with_init(&url, &init).ok()?;
    let mut resp = Fetch::Request(req).send().await.ok()?;

    if resp.status_code() != 200 {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    if body["status"].as_str() != Some("active") {
        return None;
    }

    Some(user_id)
}

/// Extracts the Bearer token and verifies it with Clerk.
/// Returns `Ok(clerk_user_id)` or an HTTP 401 error.
async fn authenticate(
    req: &Request,
    env: &Env,
) -> std::result::Result<String, Response> {
    let secret = env
        .secret("CLERK_SECRET_KEY")
        .map(|s| s.to_string())
        .map_err(|_| {
            error_response("Server misconfiguration: missing CLERK_SECRET_KEY", 500).unwrap()
        })?;

    let auth_header = req
        .headers()
        .get("Authorization")
        .ok()
        .flatten()
        .unwrap_or_default();

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| error_response("Missing Authorization header", 401).unwrap())?;

    verify_clerk_token(token, &secret)
        .await
        .ok_or_else(|| error_response("Invalid or expired session", 401).unwrap())
}

// ── Health ─────────────────────────────────────────────────────────────────

pub async fn health(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    json_response(&json!({"status": "ok", "auth": "clerk"}), 200)
}

// ── User profile (first-login upsert) ─────────────────────────────────────

pub async fn upsert_user(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let body: UpsertUserRequest = req.json().await.unwrap_or(UpsertUserRequest {
        default_currency: None,
    });
    let currency = body.default_currency.unwrap_or_else(|| "USD".to_string());

    let db = ctx.env.d1("DB")?;
    db.prepare(
        "INSERT INTO users (id, default_currency) VALUES (?1, ?2) \
         ON CONFLICT(id) DO UPDATE SET default_currency = ?2",
    )
    .bind(&[user_id.clone().into(), currency.clone().into()])?
    .run()
    .await?;

    json_response(&ApiResponse::ok(json!({"id": user_id, "default_currency": currency})), 200)
}

pub async fn get_me(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let db = ctx.env.d1("DB")?;
    match db
        .prepare("SELECT * FROM users WHERE id = ?1")
        .bind(&[user_id.into()])?
        .first::<User>(None)
        .await?
    {
        Some(u) => json_response(&ApiResponse::ok(u), 200),
        None => error_response("User not found", 404),
    }
}

// ── Expenses ───────────────────────────────────────────────────────────────

pub async fn list_expenses(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let db = ctx.env.d1("DB")?;
    let expenses: Vec<Expense> = db
        .prepare(
            "SELECT * FROM expenses WHERE user_id = ?1 AND soft_deleted = 0 ORDER BY date DESC",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results()?;
    json_response(&ApiResponse::ok(expenses), 200)
}

pub async fn create_expense(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let body: CreateExpenseRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid request body", 400),
    };

    if let Err(e) = validation::validate_amount_cents(body.amount_cents) {
        return error_response(&e.to_string(), 400);
    }
    if let Err(e) = validation::validate_date(&body.date) {
        return error_response(&e.to_string(), 400);
    }

    let db = ctx.env.d1("DB")?;
    let id = uuid::Uuid::new_v4().to_string();
    let currency = body.currency.unwrap_or_else(|| "USD".to_string());
    let description = body.description.unwrap_or_default();

    db.prepare(
        "INSERT INTO expenses (id, user_id, amount_cents, currency, category_id, description, date) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&[
        id.clone().into(),
        user_id.into(),
        body.amount_cents.into(),
        currency.into(),
        body.category_id
            .as_deref()
            .map(JsValue::from_str)
            .map(Into::into)
            .unwrap_or(JsValue::NULL.into()),
        description.into(),
        body.date.into(),
    ])?
    .run()
    .await?;

    if let Some(tag_ids) = &body.tag_ids {
        for tag_id in tag_ids {
            db.prepare("INSERT OR IGNORE INTO expense_tags (expense_id, tag_id) VALUES (?1, ?2)")
                .bind(&[id.clone().into(), tag_id.clone().into()])?
                .run()
                .await?;
        }
    }

    json_response(&ApiResponse::ok(json!({"id": id})), 201)
}

pub async fn get_expense(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let id = ctx.param("id").unwrap().to_string();
    let db = ctx.env.d1("DB")?;
    match db
        .prepare("SELECT * FROM expenses WHERE id = ?1 AND user_id = ?2 AND soft_deleted = 0")
        .bind(&[id.into(), user_id.into()])?
        .first::<Expense>(None)
        .await?
    {
        Some(e) => json_response(&ApiResponse::ok(e), 200),
        None => error_response("Expense not found", 404),
    }
}

pub async fn update_expense(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let id = ctx.param("id").unwrap().to_string();
    let body: UpdateExpenseRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid request body", 400),
    };

    if let Some(cents) = body.amount_cents {
        if let Err(e) = validation::validate_amount_cents(cents) {
            return error_response(&e.to_string(), 400);
        }
    }

    let db = ctx.env.d1("DB")?;
    // Only update provided fields
    db.prepare(
        "UPDATE expenses SET \
         amount_cents = COALESCE(?1, amount_cents), \
         currency     = COALESCE(?2, currency), \
         category_id  = COALESCE(?3, category_id), \
         description  = COALESCE(?4, description), \
         date         = COALESCE(?5, date), \
         updated_at   = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?6 AND user_id = ?7",
    )
    .bind(&[
        body.amount_cents.map(|v| JsValue::from_f64(v as f64)).unwrap_or(JsValue::NULL).into(),
        body.currency.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL).into(),
        body.category_id.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL).into(),
        body.description.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL).into(),
        body.date.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL).into(),
        id.clone().into(),
        user_id.into(),
    ])?
    .run()
    .await?;

    json_response(&ApiResponse::ok(json!({"updated": true})), 200)
}

pub async fn delete_expense(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let id = ctx.param("id").unwrap().to_string();
    let db = ctx.env.d1("DB")?;
    db.prepare(
        "UPDATE expenses SET soft_deleted = 1, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?1 AND user_id = ?2",
    )
    .bind(&[id.into(), user_id.into()])?
    .run()
    .await?;
    json_response(&ApiResponse::ok(json!({"deleted": true})), 200)
}

// ── Categories ─────────────────────────────────────────────────────────────

pub async fn list_categories(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let db = ctx.env.d1("DB")?;
    let cats: Vec<Category> = db
        .prepare(
            "SELECT * FROM categories WHERE user_id = ?1 AND soft_deleted = 0 ORDER BY name ASC",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results()?;
    json_response(&ApiResponse::ok(cats), 200)
}

pub async fn create_category(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let body: CreateCategoryRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid request body", 400),
    };
    if let Err(e) = validation::validate_name(&body.name) {
        return error_response(&e.to_string(), 400);
    }
    if let Err(e) = validation::validate_emoji(&body.emoji) {
        return error_response(&e.to_string(), 400);
    }

    let db = ctx.env.d1("DB")?;
    let id = uuid::Uuid::new_v4().to_string();
    db.prepare("INSERT INTO categories (id, user_id, name, emoji) VALUES (?1, ?2, ?3, ?4)")
        .bind(&[id.clone().into(), user_id.into(), body.name.into(), body.emoji.into()])?
        .run()
        .await?;
    json_response(&ApiResponse::ok(json!({"id": id})), 201)
}

pub async fn update_category(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let id = ctx.param("id").unwrap().to_string();
    let body: UpdateCategoryRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid request body", 400),
    };
    let db = ctx.env.d1("DB")?;
    db.prepare(
        "UPDATE categories SET \
         name = COALESCE(?1, name), \
         emoji = COALESCE(?2, emoji), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?3 AND user_id = ?4",
    )
    .bind(&[
        body.name.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL).into(),
        body.emoji.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL).into(),
        id.into(),
        user_id.into(),
    ])?
    .run()
    .await?;
    json_response(&ApiResponse::ok(json!({"updated": true})), 200)
}

pub async fn delete_category(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let id = ctx.param("id").unwrap().to_string();
    let db = ctx.env.d1("DB")?;
    db.prepare(
        "UPDATE categories SET soft_deleted = 1, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?1 AND user_id = ?2",
    )
    .bind(&[id.into(), user_id.into()])?
    .run()
    .await?;
    json_response(&ApiResponse::ok(json!({"deleted": true})), 200)
}

// ── Tags ───────────────────────────────────────────────────────────────────

pub async fn list_tags(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let db = ctx.env.d1("DB")?;
    let tags: Vec<Tag> = db
        .prepare("SELECT * FROM tags WHERE user_id = ?1 AND soft_deleted = 0 ORDER BY name ASC")
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results()?;
    json_response(&ApiResponse::ok(tags), 200)
}

pub async fn create_tag(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let body: CreateTagRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid request body", 400),
    };
    if let Err(e) = validation::validate_name(&body.name) {
        return error_response(&e.to_string(), 400);
    }
    let db = ctx.env.d1("DB")?;
    let id = uuid::Uuid::new_v4().to_string();
    db.prepare("INSERT INTO tags (id, user_id, name) VALUES (?1, ?2, ?3)")
        .bind(&[id.clone().into(), user_id.into(), body.name.into()])?
        .run()
        .await?;
    json_response(&ApiResponse::ok(json!({"id": id})), 201)
}

pub async fn delete_tag(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let id = ctx.param("id").unwrap().to_string();
    let db = ctx.env.d1("DB")?;
    db.prepare(
        "UPDATE tags SET soft_deleted = 1, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?1 AND user_id = ?2",
    )
    .bind(&[id.into(), user_id.into()])?
    .run()
    .await?;
    json_response(&ApiResponse::ok(json!({"deleted": true})), 200)
}

// ── Sync ───────────────────────────────────────────────────────────────────

pub async fn sync(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_id = match authenticate(&req, &ctx.env).await {
        Ok(id) => id,
        Err(r) => return Ok(r),
    };
    let body: SyncRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return error_response("Invalid request body", 400),
    };
    let db = ctx.env.d1("DB")?;

    let expenses: Vec<Expense> = db
        .prepare("SELECT * FROM expenses WHERE user_id = ?1 AND updated_at > ?2")
        .bind(&[user_id.clone().into(), body.last_synced_at.clone().into()])?
        .all()
        .await?
        .results()?;

    let categories: Vec<Category> = db
        .prepare("SELECT * FROM categories WHERE user_id = ?1 AND updated_at > ?2")
        .bind(&[user_id.clone().into(), body.last_synced_at.clone().into()])?
        .all()
        .await?
        .results()?;

    let tags: Vec<Tag> = db
        .prepare("SELECT * FROM tags WHERE user_id = ?1 AND updated_at > ?2")
        .bind(&[user_id.into(), body.last_synced_at.into()])?
        .all()
        .await?
        .results()?;

    json_response(
        &ApiResponse::ok(SyncResponse {
            expenses,
            categories,
            tags,
            server_time: chrono::Utc::now().to_rfc3339(),
        }),
        200,
    )
}
