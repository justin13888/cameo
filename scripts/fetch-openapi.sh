#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SDK_DIR="$SCRIPT_DIR/../openapi"
mkdir -p "$SDK_DIR"

echo "Downloading official TMDB OpenAPI spec..."
curl -fsSL https://developer.themoviedb.org/openapi/tmdb-api.json -o /tmp/tmdb-api-raw.json

echo "Raw spec version: $(jq -r '.openapi // .swagger' /tmp/tmdb-api-raw.json)"
echo "Path count: $(jq '.paths | length' /tmp/tmdb-api-raw.json)"

echo "Downgrading 3.1.0 → 3.0.x..."
npx -y @apiture/openapi-down-convert --input /tmp/tmdb-api-raw.json > "$SDK_DIR/tmdb-api.json"

echo "Converted spec version: $(jq -r '.openapi' "$SDK_DIR/tmdb-api.json")"
echo "Path count: $(jq '.paths | length' "$SDK_DIR/tmdb-api.json")"
echo "Spec written to $SDK_DIR/tmdb-api.json"
