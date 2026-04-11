---
description: "Use when writing or modifying Rust source files. Covers Cloudflare Workers WASM constraints, conditional compilation, and error handling patterns."
applyTo: "src/**/*.rs"
---
# Rust Conventions

- Target is `wasm32-unknown-unknown` — no `std::fs`, `std::net`, `std::time`, or threads
- Use `js_sys::Date::now()` for current time in WASM context
- Gate WASM-only code with `#[cfg(target_arch = "wasm32")]`
- Keep `validation.rs` free of Worker/WASM deps so `cargo test` runs natively
- Use `worker::Result<Response>` as handler return type
- Errors: return `error_response(msg, status)` — never panic in handlers
- Serialize with `serde_json`; derive `Serialize`/`Deserialize` on model structs
- Amounts: always `i64` cents, never `f64`
- IDs: `uuid::Uuid::new_v4().to_string()`
- Strings that cross into JS bindings use `JsValue::from_str()` or `.into()`
- Optional fields in D1 bind params: use `JsValue::NULL` for `None`
- Clone before `.into()` if the value is used again (Rust ownership)
