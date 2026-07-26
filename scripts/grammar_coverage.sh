#!/usr/bin/env bash
# Measures Blazelint's grammar coverage against real Ballerina source.
#
# Downloads a sample of the official "Ballerina by Example" corpus and reports
# how many files parse without any grammar-level diagnostic. Parse errors are
# grammar gaps; semantic/lint diagnostics are reported separately because those
# stages are intentionally shallow (parse-tolerant design).
#
# Usage: scripts/grammar_coverage.sh [sample-size]   (default 163)
set -euo pipefail

SAMPLE_SIZE="${1:-163}"
REPO="ballerina-platform/ballerina-distribution"
CORPUS="${CORPUS_DIR:-target/grammar-corpus}"
BIN="target/release/blazelint"

# Diagnostics that indicate a grammar (parse/lex) gap rather than a semantic one.
# Anchored to the "Error:" message line so that neither file paths (which may
# contain words like "panic") nor semantic messages (e.g. "expected int, found
# string") are miscounted as parse failures.
PARSE_ERROR_RE='^Error: (Unexpected token|Unexpected character|Unexpected end of input|Expected |Invalid assignment target|Unterminated|Malformed)'

mkdir -p "$CORPUS"

if [ ! -f "$CORPUS/filelist.txt" ]; then
    echo "Fetching example file list from $REPO..."
    curl -sS "https://api.github.com/repos/$REPO/git/trees/master?recursive=1" \
        | grep -oE '"path": "examples/[^"]+\.bal"' \
        | sed 's/"path": "//;s/"//' \
        > "$CORPUS/filelist.txt"
fi

awk -v n="$SAMPLE_SIZE" 'NR % 4 == 1 { print } NR > n * 4 { exit }' \
    "$CORPUS/filelist.txt" | head -n "$SAMPLE_SIZE" > "$CORPUS/sample.txt"

echo "Downloading corpus (up to $SAMPLE_SIZE files)..."
while read -r rel; do
    fname="${rel//\//_}"
    [ -f "$CORPUS/$fname" ] && continue
    curl -sS "https://raw.githubusercontent.com/$REPO/master/$rel" -o "$CORPUS/$fname" || true
done < "$CORPUS/sample.txt"

cargo build --release --quiet

parse_clean=0
fully_clean=0
total=0
: > "$CORPUS/failures.txt"

for f in "$CORPUS"/*.bal; do
    total=$((total + 1))
    out="$("$BIN" "$f" 2>&1 || true)"
    if echo "$out" | grep -qE "$PARSE_ERROR_RE"; then
        {
            echo "### $(basename "$f")"
            echo "$out" | grep -E "$PARSE_ERROR_RE" | head -3
        } >> "$CORPUS/failures.txt"
    else
        parse_clean=$((parse_clean + 1))
    fi
    echo "$out" | grep -qiE "error:" || fully_clean=$((fully_clean + 1))
done

echo
echo "Grammar coverage (files with no parse errors): $parse_clean / $total ($((parse_clean * 100 / total))%)"
echo "Files with no diagnostics at all:              $fully_clean / $total ($((fully_clean * 100 / total))%)"
echo
echo "Parse failures (if any) are listed in $CORPUS/failures.txt"
