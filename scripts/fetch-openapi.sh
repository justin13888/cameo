#!/usr/bin/env bash
#
# Regenerate openapi/tmdb-api.json from the official TMDB OpenAPI spec.
#
# The pipeline is fully deterministic and requires no manual post-editing:
#   1. download the official OpenAPI 3.1 spec
#   2. apply scripts/tmdb-spec-patch.jq (rename _id, fix vote_average / bool
#      defaults, strip examples, drop property defaults) — see that file for
#      the rationale of each fix
#
# The spec stays OpenAPI 3.1: spargen (the generator `cargo xtask codegen`
# runs) reads 3.1 natively and rejects 3.0, so there is no down-convert step.
#
# Given the same upstream spec, re-running this script reproduces
# openapi/tmdb-api.json byte-for-byte. After regenerating, run
# `cargo xtask codegen` to regenerate the client code (a plain `cargo build`
# does no code generation) and commit both the spec and src/generated/.
#
# Prerequisites: curl, jq.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SPEC_DIR="$SCRIPT_DIR/../openapi"
PATCH="$SCRIPT_DIR/tmdb-spec-patch.jq"
mkdir -p "$SPEC_DIR"

RAW="$(mktemp)"
trap 'rm -f "$RAW"' EXIT

echo "Downloading official TMDB OpenAPI spec..."
curl -fsSL https://developer.themoviedb.org/openapi/tmdb-api.json -o "$RAW"
echo "  raw version: $(jq -r '.openapi // .swagger' "$RAW"), paths: $(jq '.paths | length' "$RAW")"

echo "Applying deterministic patches (scripts/tmdb-spec-patch.jq)..."
jq -f "$PATCH" "$RAW" > "$SPEC_DIR/tmdb-api.json"

echo "Done: $SPEC_DIR/tmdb-api.json"
echo "  version: $(jq -r '.openapi' "$SPEC_DIR/tmdb-api.json"), paths: $(jq '.paths | length' "$SPEC_DIR/tmdb-api.json")"
echo "  size:    $(wc -c < "$SPEC_DIR/tmdb-api.json") bytes"
echo
echo "Next: run 'cargo xtask codegen' to regenerate src/generated/tmdb.rs, then"
echo "commit openapi/tmdb-api.json and src/generated/ together."
