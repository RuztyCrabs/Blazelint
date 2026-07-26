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
| **Syntactic productions parsed into structured AST** | **391 / 419 (93%)** |
| Syntactic productions *accepted* (incl. opaque blocks) | 419 / 419 (100%) |
| Real-world files parsed without grammar errors | 162 / 163 (99%) |
| False positives vs. the official compiler | **0** |
| Files with *no* diagnostics of any kind | 99 / 163 (60%) |

Read those first two rows together. Every production in the specification is
**accepted** — nothing in the grammar causes a parse error. But 28 of them are
accepted as *opaque blocks* rather than decomposed into AST nodes (§3), so the
honest implementation figure is **93%**, not 100%.

The last row is the other counterweight: the **parser** is essentially complete,
but the **semantic analyser** is deliberately shallow, so 40% of real-world
files still draw a semantic or lint diagnostic. That is where the remaining work
is — not in the grammar.

### What the measurement does and does not prove

- It proves each construct **parses without a grammar error**.
- It does **not** verify AST shape. A silent mis-parse — accepted but structured
  wrongly — would not be caught.
- Of the 419 productions, **275 (65%) are exercised by their own snippet**; the
  remaining **144 (34%) are inferred** from a parent construct that structurally
  contains them (e.g. `required-param` is inferred from `function f(int a) { }`).
  That inference is sound for true sub-parts but is weaker than direct testing.

## 2. How the official grammar was counted

The spec defines **524** productions (`<span class="ntdfn">` anchors on the spec
page). They split into four groups, which must be counted separately or the
result is meaningless:

| Group | Count | Meaning | Blazelint |
|---|---:|---|---|
| **Syntactic** | 419 | The parser grammar proper | **391 structured (93%)**, 28 opaque |
| Lexical | 63 | Character classes, escapes, number/string forms | Handled by the lexer, not as productions |
| Regex sub-grammar | 36 | The mini-language inside `` re `…` `` | Captured verbatim, not decomposed |
| Documentation | 6 | Markdown doc-comment internals | Skipped as comments |

Counting all 524 as "the grammar" would understate coverage, because 105 of them
are lexical/regex/doc productions that a linter has no reason to decompose.
Counting only the 419 syntactic productions is the meaningful comparison.

## 3. The 28 opaque productions

Four sub-grammars are **accepted but not decomposed**, because no current lint
rule inspects their internals. They parse, but the parser does not implement
their grammar — it consumes a balanced block. Consequently they also accept
malformed input:

| Sub-grammar | Productions | Accepts, though invalid |
|---|---:|---|
| Object **type** bodies | 15 | `type T object { return return return };` |
| Annotation declarations | 8 | `annotation A on on on ;` |
| `fork` bodies | 1 | `fork { return 1 2 3 }` |
| Tagged templates (regex) | 4 | ``re `((((` `` |

Full list: `annot-attach-point`, `annot-attach-points`, `annot-tag`,
`annotation-decl`, `data-tag`, `dual-attach-point`, `dual-attach-point-ident`,
`fork-stmt`, `method-decl`, `method-name`, `method-quals`,
`object-field-descriptor`, `object-member-descriptor`, `object-network-qual`,
`object-type-descriptor`, `object-type-quals`, `remote-method-decl`,
`remote-method-name`, `remote-method-quals`, `remote-qual`,
`resource-method-decl`, `resource-method-name`, `resource-method-quals`,
`resource-path`, `resource-qual`, `source-only-attach-point`,
`source-only-attach-point-ident`, `tagged-data-template-expr`.

These are design decisions rather than defects — but they should not be counted
as implemented, which is why the headline figure is 391/419.

**Note the boundary**: object *type* bodies are opaque, but object
**constructors** (`object { … }` in expression position) and `class` bodies
**are** parsed into members, so methods there are analysed. Equivalently,
`record`, `class`, and `match` bodies all correctly *reject* malformed members.

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

Production counts are those *accepted*; the opaque ones from §3 fall mainly in
"Type descriptors" (object type bodies) and "Module declarations" (annotations).

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
- 28 productions are accepted as opaque blocks and would not reject malformed
  input (§3).
- 144 of 419 productions are inferred from a parent construct rather than
  exercised directly (§1).
- The coverage test asserts *no parse error*, not AST correctness; a silent
  mis-parse would not be detected.
- Per-production coverage proves each construct **parses**. It does not claim
  the resulting AST is semantically analysed — see the 60% figure in §1.
