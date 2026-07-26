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
#   1. `bal scan` compiles the package first: full type checking, symbol
#      resolution, and cross-file analysis. Blazelint lexes, parses, and runs a
#      deliberately shallow single-file semantic pass. Scan therefore does
#      strictly more work, and finds errors Blazelint cannot.
#   2. `bal scan` pays JVM startup on every invocation. This script reports it
#      separately so it can be excluded or included knowingly.
#   3. Scan operates on a package; Blazelint on a file at a time.
#
#   The defensible claim from this harness is about *whole-tool wall-clock for an
#   equivalent rule set*, not about rule-engine throughput in isolation.
#
# Usage: scripts/benchmark_vs_scan.sh [runs]   (default 3)
set -uo pipefail

RUNS="${1:-3}"
BAL="${BAL_HOME:-$PWD/target/ballerina-dist/ballerina-2201.10.0-swan-lake}/bin/bal"
BLZ="$PWD/target/release/blazelint"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

[ -x "$BAL" ] || { echo "Ballerina not found at $BAL (set BAL_HOME)"; exit 1; }
cargo build --release --quiet

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

# --------------------------------------------------------------- measure tools
median() { sort -n | awk '{a[NR]=$1} END {print (NR%2) ? a[(NR+1)/2] : (a[NR/2]+a[NR/2+1])/2}'; }

# JVM floor: what `bal` costs before doing any analysis at all.
jvm_times=()
for _ in $(seq 1 "$RUNS"); do
    s=$(date +%s%N); (cd "$PKG" && timeout 300 "$BAL" version >/dev/null 2>&1); e=$(date +%s%N)
    jvm_times+=( $(( (e - s) / 1000000 )) )
done
JVM=$(printf '%s\n' "${jvm_times[@]}" | median)

scan_times=()
for _ in $(seq 1 "$RUNS"); do
    s=$(date +%s%N); (cd "$PKG" && timeout 900 "$BAL" scan >/dev/null 2>&1); e=$(date +%s%N)
    scan_times+=( $(( (e - s) / 1000000 )) )
done
SCAN=$(printf '%s\n' "${scan_times[@]}" | median)

blz_times=()
for _ in $(seq 1 "$RUNS"); do
    s=$(date +%s%N)
    (cd "$PKG" && for f in *.bal; do "$BLZ" "$f" >/dev/null 2>&1; done)
    e=$(date +%s%N)
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
echo "Caveat: scan type-checks the whole package; blazelint does not."
echo "See the header of this script before quoting either number."
