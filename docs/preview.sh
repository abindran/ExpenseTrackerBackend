#!/bin/bash
# Preview API docs — Swagger UI with interactive "Try it out"
# Usage: ./docs/preview.sh

set -e
cd "$(dirname "$0")/.."

echo "Starting local server for Swagger UI..."
echo "Open http://localhost:9000/docs/swagger.html"
npx http-server . -p 9000 -o docs/swagger.html
