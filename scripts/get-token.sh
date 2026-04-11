#!/usr/bin/env bash
# Fetch a Clerk JWT for local API testing.
# Usage: ./scripts/get-token.sh [user_id]
#   If user_id is omitted, picks the first user in your Clerk instance.

set -euo pipefail

DEVVARS=".dev.vars"

if [[ ! -f "$DEVVARS" ]]; then
  echo "Error: $DEVVARS not found. Create it with CLERK_SECRET_KEY=sk_test_..." >&2
  exit 1
fi

CLERK_SECRET_KEY=$(grep '^CLERK_SECRET_KEY=' "$DEVVARS" | cut -d'=' -f2-)

if [[ -z "$CLERK_SECRET_KEY" ]]; then
  echo "Error: CLERK_SECRET_KEY not found in $DEVVARS" >&2
  exit 1
fi

AUTH="Authorization: Bearer $CLERK_SECRET_KEY"

# --- Resolve user_id ---
if [[ -n "${1:-}" ]]; then
  USER_ID="$1"
else
  USER_ID=$(curl -s "https://api.clerk.com/v1/users?limit=1" \
    -H "$AUTH" | python3 -c "import sys,json; print(json.load(sys.stdin)[0]['id'])")
  echo "Using user: $USER_ID" >&2
fi

# --- Find active session ---
SESSION_ID=$(curl -s "https://api.clerk.com/v1/sessions?user_id=$USER_ID&status=active" \
  -H "$AUTH" | python3 -c "
import sys, json
data = json.load(sys.stdin)
if isinstance(data, dict) and 'errors' in data:
    print('ERROR:' + data['errors'][0]['message'], file=sys.stderr)
    sys.exit(1)
if not data:
    print('No active sessions found. Sign in via the Clerk Account Portal first.', file=sys.stderr)
    sys.exit(1)
print(data[0]['id'])
")

echo "Using session: $SESSION_ID" >&2

# --- Generate JWT ---
TOKEN=$(curl -s -X POST "https://api.clerk.com/v1/sessions/$SESSION_ID/tokens" \
  -H "$AUTH" \
  -H "Content-Type: application/json" | python3 -c "import sys,json; print(json.load(sys.stdin)['jwt'])")

echo ""
echo "Bearer $TOKEN"
