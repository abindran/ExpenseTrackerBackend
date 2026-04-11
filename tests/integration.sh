#!/bin/bash
# Integration tests for the Expense Tracker API.
# Requires a running dev server: `npx wrangler dev`
#
# Usage:
#   ./tests/integration.sh                          # Uses http://localhost:8787
#   ./tests/integration.sh https://your-worker.dev  # Custom base URL
#   CLERK_JWT="eyJ..." ./tests/integration.sh       # With auth token

set -euo pipefail

BASE_URL="${1:-http://localhost:8787}"
PASS=0
FAIL=0
SKIP=0

# ── Helpers ─────────────────────────────────────────────────────

green()  { printf "\033[32m%s\033[0m\n" "$1"; }
red()    { printf "\033[31m%s\033[0m\n" "$1"; }
yellow() { printf "\033[33m%s\033[0m\n" "$1"; }

assert_status() {
    local name="$1" expected="$2" actual="$3"
    if [ "$actual" -eq "$expected" ]; then
        green "  ✓ $name (HTTP $actual)"
        PASS=$((PASS + 1))
    else
        red "  ✗ $name — expected $expected, got $actual"
        FAIL=$((FAIL + 1))
    fi
}

assert_json_field() {
    local name="$1" body="$2" field="$3" expected="$4"
    local actual
    actual=$(echo "$body" | python3 -c "import sys,json; print(json.load(sys.stdin).get('$field',''))" 2>/dev/null || echo "")
    if [ "$actual" = "$expected" ]; then
        green "  ✓ $name ($field=$actual)"
        PASS=$((PASS + 1))
    else
        red "  ✗ $name — expected $field=$expected, got $field=$actual"
        FAIL=$((FAIL + 1))
    fi
}

AUTH_HEADER=""
if [ -n "${CLERK_JWT:-}" ]; then
    AUTH_HEADER="Authorization: Bearer $CLERK_JWT"
fi

# ── Health endpoint (no auth required) ──────────────────────────

echo ""
echo "=== Health ==="

RESP=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
BODY=$(echo "$RESP" | head -n -1)
STATUS=$(echo "$RESP" | tail -1)

assert_status "GET /health returns 200" 200 "$STATUS"
assert_json_field "GET /health has status=ok" "$BODY" "status" "ok"
assert_json_field "GET /health has auth=clerk" "$BODY" "auth" "clerk"

# ── Unauthenticated access returns 401 ──────────────────────────

echo ""
echo "=== Auth (unauthenticated) ==="

for endpoint in "/api/users/me" "/api/expenses" "/api/categories" "/api/tags"; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL$endpoint")
    assert_status "GET $endpoint without token returns 401" 401 "$STATUS"
done

# ── Bad Bearer token returns 401 ────────────────────────────────

echo ""
echo "=== Auth (bad token) ==="

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer invalid.token.here" \
    "$BASE_URL/api/users/me")
assert_status "GET /api/users/me with bad token returns 401" 401 "$STATUS"

# ── Authenticated endpoints (only if CLERK_JWT is set) ──────────

if [ -z "${CLERK_JWT:-}" ]; then
    echo ""
    yellow "=== Skipping authenticated tests (set CLERK_JWT env var to enable) ==="
    SKIP=$((SKIP + 8))
else
    echo ""
    echo "=== User Profile ==="

    # Upsert user
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"default_currency":"USD"}' \
        "$BASE_URL/api/users/me")
    BODY=$(echo "$RESP" | head -n -1)
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/users/me returns 200" 200 "$STATUS"
    assert_json_field "POST /api/users/me success=True" "$BODY" "success" "True"

    # Get user
    RESP=$(curl -s -w "\n%{http_code}" \
        -H "$AUTH_HEADER" \
        "$BASE_URL/api/users/me")
    BODY=$(echo "$RESP" | head -n -1)
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "GET /api/users/me returns 200" 200 "$STATUS"
    assert_json_field "GET /api/users/me success=True" "$BODY" "success" "True"

    echo ""
    echo "=== Categories ==="

    # Create category
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"name":"Test Category","emoji":"🧪"}' \
        "$BASE_URL/api/categories")
    BODY=$(echo "$RESP" | head -n -1)
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/categories returns 201" 201 "$STATUS"
    assert_json_field "POST /api/categories success=True" "$BODY" "success" "True"

    # List categories
    RESP=$(curl -s -w "\n%{http_code}" \
        -H "$AUTH_HEADER" \
        "$BASE_URL/api/categories")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "GET /api/categories returns 200" 200 "$STATUS"

    echo ""
    echo "=== Tags ==="

    # Create tag
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"name":"test-tag"}' \
        "$BASE_URL/api/tags")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/tags returns 201" 201 "$STATUS"

    # List tags
    RESP=$(curl -s -w "\n%{http_code}" \
        -H "$AUTH_HEADER" \
        "$BASE_URL/api/tags")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "GET /api/tags returns 200" 200 "$STATUS"

    echo ""
    echo "=== Expenses ==="

    # Create expense
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"amount_cents":1599,"date":"2026-04-12","description":"Integration test expense"}' \
        "$BASE_URL/api/expenses")
    BODY=$(echo "$RESP" | head -n -1)
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/expenses returns 201" 201 "$STATUS"
    assert_json_field "POST /api/expenses success=True" "$BODY" "success" "True"

    # Extract expense ID
    EXPENSE_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('data',{}).get('id',''))" 2>/dev/null || echo "")

    # List expenses
    RESP=$(curl -s -w "\n%{http_code}" \
        -H "$AUTH_HEADER" \
        "$BASE_URL/api/expenses")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "GET /api/expenses returns 200" 200 "$STATUS"

    # Get single expense
    if [ -n "$EXPENSE_ID" ]; then
        RESP=$(curl -s -w "\n%{http_code}" \
            -H "$AUTH_HEADER" \
            "$BASE_URL/api/expenses/$EXPENSE_ID")
        STATUS=$(echo "$RESP" | tail -1)
        assert_status "GET /api/expenses/:id returns 200" 200 "$STATUS"

        # Update expense
        RESP=$(curl -s -w "\n%{http_code}" \
            -X PUT -H "Content-Type: application/json" \
            -H "$AUTH_HEADER" \
            -d '{"amount_cents":2000}' \
            "$BASE_URL/api/expenses/$EXPENSE_ID")
        STATUS=$(echo "$RESP" | tail -1)
        assert_status "PUT /api/expenses/:id returns 200" 200 "$STATUS"

        # Delete expense
        RESP=$(curl -s -w "\n%{http_code}" \
            -X DELETE -H "$AUTH_HEADER" \
            "$BASE_URL/api/expenses/$EXPENSE_ID")
        STATUS=$(echo "$RESP" | tail -1)
        assert_status "DELETE /api/expenses/:id returns 200" 200 "$STATUS"
    fi

    echo ""
    echo "=== Validation ==="

    # Invalid amount (zero)
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"amount_cents":0,"date":"2026-04-12"}' \
        "$BASE_URL/api/expenses")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/expenses with zero amount returns 400" 400 "$STATUS"

    # Invalid date
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"amount_cents":100,"date":"not-a-date"}' \
        "$BASE_URL/api/expenses")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/expenses with bad date returns 400" 400 "$STATUS"

    # Empty category name
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"name":"","emoji":"🍔"}' \
        "$BASE_URL/api/categories")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/categories with empty name returns 400" 400 "$STATUS"

    # Empty tag name
    RESP=$(curl -s -w "\n%{http_code}" \
        -X POST -H "Content-Type: application/json" \
        -H "$AUTH_HEADER" \
        -d '{"name":""}' \
        "$BASE_URL/api/tags")
    STATUS=$(echo "$RESP" | tail -1)
    assert_status "POST /api/tags with empty name returns 400" 400 "$STATUS"
fi

# ── 404 for unknown routes ──────────────────────────────────────

echo ""
echo "=== Unknown routes ==="

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/nonexistent")
assert_status "GET /api/nonexistent returns 404" 404 "$STATUS"

# ── Summary ─────────────────────────────────────────────────────

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
green "  Passed:  $PASS"
if [ "$FAIL" -gt 0 ]; then
    red "  Failed:  $FAIL"
fi
if [ "$SKIP" -gt 0 ]; then
    yellow "  Skipped: $SKIP"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

exit "$FAIL"
