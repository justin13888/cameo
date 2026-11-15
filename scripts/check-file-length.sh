#!/usr/bin/env bash
#
# Fail when a hand-written Rust source under src/ is longer than LIMIT lines.
#
# Length is the file's total line count (`wc -l`), blank lines and comments
# included. src/generated/ is excluded by name: it holds the progenitor client
# that `cargo xtask codegen` regenerates, which nobody edits by hand.
#
# scripts/file-length-baseline.txt lists the paths still allowed over the limit,
# one per line (blank lines and `#` comments are ignored). The baseline may only
# shrink:
#   - an unlisted path over the limit fails;
#   - a listed path that no longer exists, or is now within the limit, fails
#     until its line is removed;
#   - with FILE_LENGTH_BASE_REF set (CI sets it to the pull request's base), a
#     listed path the base ref's baseline does not list fails.
#
# To change the threshold, change LIMIT below.

set -euo pipefail

readonly LIMIT=436
readonly BASELINE=scripts/file-length-baseline.txt

cd "$(dirname "$0")/.."

# Prints the baseline at $1 (a file, or `-` for stdin) without comments/blanks.
read_baseline() {
    sed -e 's/#.*$//' -e 's/[[:space:]]*$//' -e '/^$/d' "$1"
}

declare -A listed=()
if [[ -f "${BASELINE}" ]]; then
    while IFS= read -r path; do
        listed["${path}"]=1
    done < <(read_baseline "${BASELINE}")
fi

failures=0
fail() {
    echo "error: $*" >&2
    failures=$((failures + 1))
}

while IFS= read -r -d '' file; do
    path="${file#./}"
    lines=$(wc -l <"${file}")
    if ((lines > LIMIT)) && [[ -z "${listed[${path}]:-}" ]]; then
        fail "${path} has ${lines} lines (limit ${LIMIT}); split it"
    fi
done < <(find src -path src/generated -prune -o -type f -name '*.rs' -print0 | sort -z)

for path in "${!listed[@]}"; do
    if [[ ! -f "${path}" ]]; then
        fail "${BASELINE} lists ${path}, which does not exist; remove the line"
    elif [[ "${path}" == src/generated/* ]]; then
        fail "${BASELINE} lists ${path}, which is excluded already; remove the line"
    else
        lines=$(wc -l <"${path}")
        if ((lines <= LIMIT)); then
            fail "${path} is within the limit (${lines} <= ${LIMIT}); remove it from ${BASELINE}"
        fi
    fi
done

if [[ -n "${FILE_LENGTH_BASE_REF:-}" ]]; then
    if base=$(git show "${FILE_LENGTH_BASE_REF}:${BASELINE}" 2>/dev/null); then
        declare -A base_listed=()
        while IFS= read -r path; do
            base_listed["${path}"]=1
        done < <(read_baseline - <<<"${base}")
        for path in "${!listed[@]}"; do
            if [[ -z "${base_listed[${path}]:-}" ]]; then
                fail "${BASELINE} adds ${path}, which ${FILE_LENGTH_BASE_REF} does not list; the baseline may only shrink"
            fi
        done
    else
        echo "note: ${FILE_LENGTH_BASE_REF} has no ${BASELINE}; skipping the growth check"
    fi
fi

if ((failures > 0)); then
    echo "file-length: ${failures} problem(s)" >&2
    exit 1
fi
echo "file-length: every hand-written file under src/ is within ${LIMIT} lines or baselined"
