#!/usr/bin/env bash
# Compares Blazelint's diagnostics against the official Ballerina compiler.
#
# For each corpus file, runs `bal build` (ground truth) and Blazelint, then
# classifies the result:
#
#   - FALSE POSITIVE: the official compiler accepts the file but Blazelint
#     reports an error. These are Blazelint bugs.
#   - agreement:      both accept, or both reject.
#
# Files the official compiler rejects (missing dependencies, unresolved
# imports for offline builds, etc.) are excluded from the false-positive
# count, since Blazelint is not expected to match those diagnostics.
#
# Requires a local Ballerina distribution; see scripts/grammar_coverage.sh for
# the corpus. Usage: scripts/compare_with_ballerina.sh [max-files]
set -uo pipefail

MAX="${1:-40}"
CORPUS="${CORPUS_DIR:-target/grammar-corpus}"
BIN="$PWD/target/release/blazelint"
BAL="${BAL_HOME:-$PWD/target/ballerina-dist/ballerina-2201.10.0-swan-lake}/bin/bal"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

[ -x "$BAL" ] || { echo "Ballerina not found at $BAL (set BAL_HOME)"; exit 1; }
cargo build --release --quiet

both_ok=0; both_err=0; false_pos=0; bal_only=0; checked=0; skipped=0
: > "$CORPUS/false_positives.txt"

for f in "$CORPUS"/*.bal; do
    [ "$checked" -ge "$MAX" ] && break
    checked=$((checked + 1))
    name="$(basename "$f")"

    cp "$f" "$WORK/src.bal"
    # `bal build` can exit non-zero while still succeeding, so detect compiler
    # errors from its output text rather than the exit status.
    bal_out="$(cd "$WORK" && timeout 180 "$BAL" build --offline src.bal 2>&1)"
    rm -f "$WORK"/*.jar

    # Offline dependency-resolution failures are out of scope for a linter:
    # once a module cannot be resolved, every symbol from it is reported
    # undefined. Skip such files rather than counting them as disagreements.
    if echo "$bal_out" | grep -qE "cannot resolve module"; then
        skipped=$((skipped + 1))
        continue
    fi
    if echo "$bal_out" | grep -qE "^ERROR|error:|compilation errors"; then
        bal_ok=0
    else
        bal_ok=1
    fi

    if "$BIN" "$f" 2>&1 | grep -qiE "error:"; then blz_ok=0; else blz_ok=1; fi

    if [ "$bal_ok" = 1 ] && [ "$blz_ok" = 1 ]; then
        both_ok=$((both_ok + 1))
    elif [ "$bal_ok" = 1 ] && [ "$blz_ok" = 0 ]; then
        false_pos=$((false_pos + 1))
        {
            echo "### $name"
            "$BIN" "$f" 2>&1 | grep -iE "error:" | head -3
        } >> "$CORPUS/false_positives.txt"
    elif [ "$bal_ok" = 0 ] && [ "$blz_ok" = 0 ]; then
        both_err=$((both_err + 1))
    else
        bal_only=$((bal_only + 1))
    fi
done

echo
echo "Compared $((checked - skipped)) files against Ballerina $("$BAL" version 2>/dev/null | head -1)"
echo "  (skipped $skipped with unresolvable offline dependencies)"
echo "  both accept .................. $both_ok"
echo "  both reject .................. $both_err"
echo "  Blazelint FALSE POSITIVES .... $false_pos   (official compiler accepts, Blazelint errors)"
echo "  Blazelint missed an error .... $bal_only"
echo
echo "False positives detailed in $CORPUS/false_positives.txt"
