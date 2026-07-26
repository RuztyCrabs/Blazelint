#!/usr/bin/env bash
# Benchmarks Blazelint against the official `bal scan`, with the rule sets
# matched so both tools do the same analysis work.
#
# WHY MATCHING MATTERS
#   Blazelint ships 13 lint rules and 25 semantic checks; `bal scan` 0.5.0 ships
#   two rules (ballerina:1 avoid-checkpanic, ballerina:2 unused parameter). A
#   default-vs-default timing would compare different amounts of work. This
#   script disables every Blazelint rule except those two, so the comparison is
#   like-for-like at the rule level.
#
# WHAT REMAINS DIFFERENT — read before quoting a speedup
#   1. Both tools type-check; Blazelint's pass is shallower. On a four-error
#      sample the compiler caught all four and Blazelint three, missing only a
#      record *field* type — field types, lang-library method signatures, and
#      cross-file symbols resolve to Unknown here. Type checking is ~12% of
#      Blazelint's runtime, so it is real work, not a step being skipped.
#   2. Neither benchmarked rule needs type information: `checkpanic` detection is
#      syntactic and unused-parameter is scope-based. `bal scan` still compiles
#      the whole package, because it runs as a compiler plugin. That is an
#      architectural cost of their design, not work the rules require — and it
#      is also what gives their other rules type information for free.
#   3. `bal scan` pays JVM startup on every invocation, reported separately below.
#   4. Scan operates on a package; Blazelint on a file at a time.
#
#   The defensible claim is whole-tool wall-clock for an equivalent rule set,
#   with the caveat that Blazelint's type checking is shallower.
#
# Usage: scripts/benchmark_vs_scan.sh [runs]   (default 3)
set -uo pipefail

RUNS="${1:-3}"
BAL="${BAL_HOME:-$PWD/target/ballerina-dist/ballerina-2201.10.0-swan-lake}/bin/bal"
BLZ="$PWD/target/release/blazelint"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

[ -x "$BAL" ] || { echo "Ballerina not found at $BAL (set BAL_HOME)"; exit 1; }
command -v jq >/dev/null || { echo "jq is required to compare findings"; exit 1; }

# A stale or missing binary must never be benchmarked: fail here, not later.
cargo build --release --quiet || { echo "cargo build --release failed"; exit 1; }
[ -x "$BLZ" ] || { echo "blazelint binary not found at $BLZ"; exit 1; }

# ---------------------------------------------------------------- corpus setup
PKG="$WORK/bench"
mkdir -p "$PKG"
cat > "$PKG/Ballerina.toml" <<'TOML'
[package]
org = "bench"
name = "bench"
version = "0.1.0"
distribution = "2201.10.0"
TOML

# Blazelint restricted to exactly the rules `bal scan` 0.5.0 implements.
cat > "$PKG/.blazerc" <<'RC'
[rules]
avoid-checkpanic = "warn"
unused-parameters = "warn"
camel-case = "off"
constant-case = "off"
line-length = "off"
max-function-length = "off"
missing-return = "off"
unused-variables = "off"
self-assignment = "off"
invalid-range = "off"
isolated-public-function = "off"
isolated-public-method = "off"
isolated-public-class = "off"
RC

# Generate a corpus that both tools accept: self-contained, no imports, and
# exercising both rules so neither tool is idling.
FILES="${FILES:-40}"
for i in $(seq 1 "$FILES"); do
    cat > "$PKG/mod$i.bal" <<EOF
function helper$i(int a, int unusedParam$i) returns int {
    int total = a;
    foreach int j in 0 ... 9 {
        total += j;
    }
    return total;
}

function risky$i() returns error? {
    return;
}

public function run$i() {
    checkpanic risky$i();
    int _ = helper$i(1, 2);
}
EOF
done

echo "Corpus: $FILES files in a single package"
echo "Rule set: ballerina:1 (avoid-checkpanic), ballerina:2 (unused parameter)"
echo

# ------------------------------------------------------- correctness gate
# A timing comparison is only meaningful if both tools actually did the work and
# found the same things. Run each once, unsuppressed, and diff the findings
# before any number is reported.
run_or_die() { # label timeout cmd...
    local label="$1" limit="$2"; shift 2
    local out status
    out=$(timeout "$limit" "$@" 2>&1); status=$?
    if [ "$status" -eq 124 ]; then
        echo "$label timed out after ${limit}s" >&2; exit 1
    elif [ "$status" -ne 0 ]; then
        echo "$label failed (exit $status):" >&2; echo "$out" >&2; exit 1
    fi
    printf '%s\n' "$out"
}

# Canonical finding: file:line:rule-id. Columns differ between the tools'
# anchor points, so equivalence is compared at line granularity.
(cd "$PKG" && run_or_die "bal scan" 900 "$BAL" scan) >/dev/null
jq -r '.[] | "\(.location.filePath):\(.location.startLine + 1):\(.rule.id)"' \
    "$PKG/target/report/scan_results.json" | sort > "$WORK/scan.findings"

: > "$WORK/blz.findings"
for f in "$PKG"/*.bal; do
    (cd "$PKG" && run_or_die "blazelint $(basename "$f")" 300 "$BLZ" "$(basename "$f")")
done | awk '
    /^(Warning|Error):/ { msg = $0 }
    /^[[:space:]]*--> / {
        rule = "unmatched"
        if (msg ~ /never used/) rule = "ballerina:2"
        else if (msg ~ /checkpanic/) rule = "ballerina:1"
        split($2, p, ":"); print p[1] ":" p[2] ":" rule
    }' | sort > "$WORK/blz.findings"

if ! diff -u "$WORK/scan.findings" "$WORK/blz.findings" > "$WORK/findings.diff"; then
    echo "Findings differ between bal scan and blazelint; refusing to benchmark." >&2
    echo "(-) bal scan only, (+) blazelint only:" >&2
    sed -n '3,$p' "$WORK/findings.diff" >&2
    exit 1
fi
FINDINGS=$(wc -l < "$WORK/scan.findings")
[ "$FINDINGS" -gt 0 ] || { echo "No findings produced; corpus or rule set is wrong" >&2; exit 1; }
echo "Findings: $FINDINGS, identical between both tools"

# --------------------------------------------------------------- measure tools
median() { sort -n | awk '{a[NR]=$1} END {print (NR%2) ? a[(NR+1)/2] : (a[NR/2]+a[NR/2+1])/2}'; }

# Timed runs stay quiet for clean measurement, but a nonzero exit or a timeout
# invalidates the sample rather than being recorded as a fast run.
check_timed() { # label status
    if [ "$2" -eq 124 ]; then echo "$1 timed out during timing run" >&2; exit 1
    elif [ "$2" -ne 0 ]; then echo "$1 failed during timing run (exit $2)" >&2; exit 1; fi
}

# JVM floor: what `bal` costs before doing any analysis at all.
jvm_times=()
for _ in $(seq 1 "$RUNS"); do
    s=$(date +%s%N); (cd "$PKG" && timeout 300 "$BAL" version >/dev/null 2>&1); status=$?; e=$(date +%s%N)
    check_timed "bal version" "$status"
    jvm_times+=( $(( (e - s) / 1000000 )) )
done
JVM=$(printf '%s\n' "${jvm_times[@]}" | median)

scan_times=()
for _ in $(seq 1 "$RUNS"); do
    s=$(date +%s%N); (cd "$PKG" && timeout 900 "$BAL" scan >/dev/null 2>&1); status=$?; e=$(date +%s%N)
    check_timed "bal scan" "$status"
    scan_times+=( $(( (e - s) / 1000000 )) )
done
SCAN=$(printf '%s\n' "${scan_times[@]}" | median)

blz_times=()
for _ in $(seq 1 "$RUNS"); do
    s=$(date +%s%N)
    (cd "$PKG" && for f in *.bal; do timeout 300 "$BLZ" "$f" >/dev/null 2>&1 || exit $?; done)
    status=$?
    e=$(date +%s%N)
    check_timed "blazelint" "$status"
    blz_times+=( $(( (e - s) / 1000000 )) )
done
BLZ_MS=$(printf '%s\n' "${blz_times[@]}" | median)

# ------------------------------------------------------------------- reporting
echo "Median of $RUNS runs (ms):"
printf '  bal scan (total) ............ %6s\n' "$SCAN"
printf '    of which JVM startup ...... %6s\n' "$JVM"
printf '    analysis only ............. %6s\n' "$((SCAN - JVM))"
printf '  blazelint (%s invocations) ... %6s\n' "$FILES" "$BLZ_MS"
echo
if [ "$BLZ_MS" -gt 0 ]; then
    echo "  whole-tool speedup ....... $(( SCAN / BLZ_MS ))x"
    echo "  excluding JVM startup .... $(( (SCAN - JVM) / BLZ_MS ))x"
fi
echo
echo "Caveat: both tools type-check, but Blazelint's pass is shallower (it"
echo "misses record field types, lang-lib signatures, and cross-file symbols)."
echo "Neither benchmarked rule needs type information; scan compiles the package"
echo "anyway because it runs as a compiler plugin. See this script's header."
