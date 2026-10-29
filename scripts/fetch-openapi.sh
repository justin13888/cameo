#!/usr/bin/env bash
#
# Regenerate openapi/tmdb-api.json from the official TMDB OpenAPI spec.
#
# The pipeline is fully deterministic and requires no manual post-editing:
#   1. download the official 3.1.0 spec
#   2. down-convert 3.1.0 -> 3.0.3 with a PINNED @apiture/openapi-down-convert
#   3. apply scripts/tmdb-spec-patch.jq (rename _id, fix vote_average / bool
#      defaults, strip examples, drop property defaults) — see that file for
#      the rationale of each fix
#
# Given the same upstream spec, re-running this script reproduces
# openapi/tmdb-api.json byte-for-byte. After regenerating, rebuild the crate to
# regenerate the client code and commit both the spec and src/generated/.
#
# Prerequisites: curl, jq, npx (Node.js).
set -euo pipefail

# Pin the down-converter so its output cannot drift with upstream releases.
DOWN_CONVERT_VERSION="0.14.2"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPEC_DIR="$SCRIPT_DIR/../openapi"
PATCH="$SCRIPT_DIR/tmdb-spec-patch.jq"
mkdir -p "$SPEC_DIR"

RAW="$(mktemp)"
CONVERTED="$(mktemp)"
trap 'rm -f "$RAW" "$CONVERTED"' EXIT

echo "Downloading official TMDB OpenAPI spec..."
curl -fsSL https://developer.themoviedb.org/openapi/tmdb-api.json -o "$RAW"
echo "  raw version: $(jq -r '.openapi // .swagger' "$RAW"), paths: $(jq '.paths | length' "$RAW")"

echo "Down-converting 3.1.0 -> 3.0.x with openapi-down-convert@${DOWN_CONVERT_VERSION}..."
npx -y "@apiture/openapi-down-convert@${DOWN_CONVERT_VERSION}" --input "$RAW" > "$CONVERTED"

echo "Applying deterministic patches (scripts/tmdb-spec-patch.jq)..."
jq -f "$PATCH" "$CONVERTED" > "$SPEC_DIR/tmdb-api.json"

echo "Done: $SPEC_DIR/tmdb-api.json"
echo "  version: $(jq -r '.openapi' "$SPEC_DIR/tmdb-api.json"), paths: $(jq '.paths | length' "$SPEC_DIR/tmdb-api.json")"
echo "  size:    $(wc -c < "$SPEC_DIR/tmdb-api.json") bytes"
echo
echo "Next: rebuild the crate ('cargo build --features tmdb') to regenerate the"
echo "client code, then commit openapi/tmdb-api.json."
