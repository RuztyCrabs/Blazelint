# Grammar Coverage vs. the Official Ballerina Specification

How much of Ballerina's grammar Blazelint implements, measured rather than
estimated.

- **Reference**: [Ballerina Language Specification 2024R1](https://ballerina.io/spec/lang/2024R1/)
- **Validated against**: Ballerina `2201.10.0` (Swan Lake Update 10), which
  implements language spec 2024R1 — the same version this document compares to.
- **Blazelint grammar**: [`EBNF.md`](EBNF.md) (spec notation) / [`BNF.md`](BNF.md) (prose)

---

## 1. Headline numbers

| Measure | Result |
|---|---|
| **Syntactic productions implemented** | **419 / 419 (100%)** |
| Real-world files parsed without grammar errors | 162 / 163 (99%) |
| False positives vs. the official compiler | **0** |
| Files with *no* diagnostics of any kind | 99 / 163 (60%) |

The last row is the honest counterweight: the **parser** is essentially
complete, but the **semantic analyser** is deliberately shallow, so 40% of
real-world files still draw a semantic or lint diagnostic. That is where the
remaining work is — not in the grammar.

## 2. How the official grammar was counted

The spec defines **524** productions (`<span class="ntdfn">` anchors on the spec
page). They split into four groups, which must be counted separately or the
result is meaningless:

| Group | Count | Meaning | Blazelint |
|---|---:|---|---|
| **Syntactic** | 419 | The parser grammar proper | **419 implemented (100%)** |
| Lexical | 63 | Character classes, escapes, number/string forms | Handled by the lexer, not as productions |
| Regex sub-grammar | 36 | The mini-language inside `` re `…` `` | Captured verbatim, not decomposed |
| Documentation | 6 | Markdown doc-comment internals | Skipped as comments |

Counting all 524 as "the grammar" would understate coverage, because 105 of them
are lexical/regex/doc productions that a linter has no reason to decompose.
Counting only the 419 syntactic productions is the meaningful comparison.

## 3. Deliberate non-goals

Three sub-grammars are accepted but not decomposed into an AST, because no lint
rule inspects their internals:

1. **Regular expressions** (36 productions) — `` re `[a-z]+` `` is captured as a
   template. Validating regex syntax is a separate concern.
2. **Object *type* bodies** — `object { … }` in type position is consumed as a
   balanced block. Object *constructors* and `class` bodies **are** parsed into
   members, so methods are analysed.
3. **`fork` bodies** and annotation attach-point lists — consumed as balanced
   blocks.

These are design decisions, not gaps: the constructs parse, they just do not
produce structured nodes.

## 4. Reproducing the measurements

```bash
# Per-production coverage (419 snippets, one per official production)
cargo test --lib grammar_coverage_of_official_spec

# File-level coverage over the official "Ballerina by Example" corpus
bash scripts/grammar_coverage.sh

# Differential comparison against the official compiler
bash scripts/compare_with_ballerina.sh 60
```

`compare_with_ballerina.sh` needs a local Ballerina distribution; point
`BAL_HOME` at it, or let the default path
`target/ballerina-dist/ballerina-2201.10.0-swan-lake` apply.

## 5. Coverage by area

Every area below is at 100% of its syntactic productions.

| Area | Productions | Notes |
|---|---:|---|
| Type descriptors | 103 | records (open/closed/inclusion/rest), objects, tuples, function types, `error<D>`/`future<T>`/`stream<T,C>`/`typedesc<T>`/`table<R> key(…)`, unions, intersections, singletons, `distinct`, subtypes (`int:Signed32`) |
| Expressions | 124 | full precedence ladder, query expressions, closures & arrow functions, templates with interpolation, XML navigation, error constructors, object constructors |
| Statements & patterns | 96 | `match` with all pattern forms, `do`/`on fail`, `lock`, `fork`, `transaction`/`retry`/`rollback`, workers, destructuring |
| Module declarations | 61 | `type`, `class`, `enum`, `service`, `listener`, `annotation`, `xmlns`, `configurable`, qualifiers |
| Actions & concurrency | 35 | worker send/receive (`->`, `->>`, `<-`), `wait`, `flush`, `start`, remote & resource-access calls |

## 6. Notable spec-conformance findings

Two bugs were found by differential testing against the official compiler, not
by reading the spec:

1. **Ballerina has no block comments.** Blazelint previously treated `/* … */`
   as a comment (a C-family assumption). The official compiler rejects `/*` as
   `invalid token`. Beyond being wrong, it silently swallowed the XML
   all-children navigation step `x/*`. Removed.
2. **Integer division is not float division.** `int / int` yields `int` in
   Ballerina; Blazelint was widening the result to `float`, producing false
   type-mismatch diagnostics.

## 7. Known limitations

- The corpus is a 163-file sample of the 651 available examples; both the file
  and comparison harnesses need network access on first run.
- One corpus file still fails to parse: `natural-expressions`, which uses the
  recent AI `natural { … }` expression block.
- Per-production coverage proves each construct **parses**. It does not claim
  the resulting AST is semantically analysed — see the 60% figure in §1.
