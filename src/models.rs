use serde::{Deserialize, Serialize};

// ── User (app preferences only — credentials live in Clerk) ───────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String, // Clerk user_id
    pub default_currency: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpsertUserRequest {
    pub default_currency: Option<String>,
}

// ── Categories ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub emoji: String,
    pub soft_deleted: bool,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub emoji: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub emoji: Option<String>,
}

// ── Expenses ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Expense {
    pub id: String,
    pub user_id: String,
    pub amount_cents: i64,
    pub currency: String,
    pub category_id: Option<String>,
    pub description: String,
    pub date: String,
    pub soft_deleted: bool,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpenseRequest {
    pub amount_cents: i64,
    pub currency: Option<String>,
    pub category_id: Option<String>,
    pub description: Option<String>,
    pub date: String,
    pub tag_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpenseRequest {
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub category_id: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
    pub tag_ids: Option<Vec<String>>,
}

// ── Tags ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub soft_deleted: bool,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

// ── Sync ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub last_synced_at: String,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub expenses: Vec<Expense>,
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
    pub server_time: String,
}

// ── API Response Wrapper ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn err(message: String) -> Self {
        Self { success: false, data: None, error: Some(message) }
    }
}
