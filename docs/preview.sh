#!/bin/bash
# Build and open the OpenAPI spec as interactive HTML docs
# Usage: ./docs/preview.sh

set -e
cd "$(dirname "$0")/.."

OUT="docs/index.html"
echo "Building API docs..."
npx @redocly/cli build-docs docs/openapi.yaml -o "$OUT"
echo "Opening $OUT in browser..."
open "$OUT"
