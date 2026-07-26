# Plan: Bring Diagnostics Up to the Parsed Grammar

**Goal** — make semantic analysis and lint rules cover the 93% of the grammar the
parser now structures, so Blazelint's *output* is trustworthy on real code, not
just its parsing.

**Current state** — 99/163 corpus files (60%) are diagnostic-free. The other 64
produce **207 diagnostics, and the sampled ones are all false positives**: the
official Ballerina 2201.10.0 compiler accepts every file Blazelint rejects.

**Target** — ≥95% of corpus files diagnostic-free, with zero false positives
maintained, and every parser AST node reachable by both analysis passes.

---

## 1. Why the false positives happen

Measured by running the corpus and grouping every diagnostic by root cause:

| # | Root cause | Diagnostics | Share |
|---|---|---:|---|
| 1 | **Named arguments modelled as assignments** | ~114 | 55% |
| 2 | **Class/service method bodies never analysed** | (silent) | — |
| 3 | Function names not usable as values | ~15 | 7% |
| 4 | `can_compare` rejects structured types | 11 | 5% |
| 5 | Parameters flagged as unused variables | ~30 | 14% |
| 6 | Worker names not bound as symbols | ~6 | 3% |
| 7 | `error?`-returning functions want a return value | 9 | 4% |

Each is reproducible as a one-liner:

```ballerina
function g(int a) {}  function f() { g(a = 1); }   // #1 undeclared variable 'a'
function g() {}       function f() { var t = [g]; } // #3 undeclared variable 'g'
function f() { int[] a = [1]; int[] b = [2]; boolean c = a == b; } // #4
function f(int unusedParam) { }                     // #5 "never used"
function f() { worker w1 { } int x = <- w1; }       // #6 undeclared variable 'w1'
```

Cause #2 is the most consequential and is *silent* — it suppresses diagnostics
rather than inventing them:

```ballerina
class C {
  function m() {
    int x = "type error";   // not reported
    undeclaredThing();      // not reported
  }
}
```

`Stmt::ClassDef` and `Stmt::ServiceDecl` are no-ops in `check_stmt`, so every
method body in a class or service is unanalysed. Services are the dominant
Ballerina idiom, so this is a large blind spot.

## 2. Phases

Ordered by (diagnostics eliminated) ÷ (effort). Each phase ends green on
`scripts/check.sh` and re-measures the corpus.

### Phase A — Model named arguments correctly *(fixes ~55% of diagnostics)*

The parser currently turns `g(a = 1)` into an `Expr::Assign`, so the analyser
resolves `a` as a variable. Named arguments are a distinct construct.

- `src/ast.rs` — add `Expr::NamedArg { name: String, name_span: Span, value: Box<Expr>, span: Span }`.
- `src/parser.rs` — in the four argument loops (`finish_call`, method call,
  qualified call, remote call), when the next tokens are `IDENT "="` **and** the
  `=` is not `==`, emit `NamedArg` instead of parsing an assignment.
- `src/semantic.rs` — `check_expr` returns the value's type without resolving
  `name`. Later (Phase F) it can be checked against the callee's parameter names.
- `unused_variables.rs` — visit `value` only; the name is not a reference.

*Exit check*: `g(a = 1)` clean; `a = 1` as a statement still reports assignment
to an undeclared variable.

### Phase B — Analyse class/service/object bodies *(removes the silent blind spot)*

- `src/semantic.rs` — replace the `ClassDef | ServiceDecl => {}` arm with a walk
  that, per member, pushes a scope binding `self`, binds fields as symbols, and
  runs `check_stmt` over method bodies with the method's return type as context.
  Reuse the existing `Stmt::Function` arm rather than duplicating it.
- Do the same for `Expr::ObjectConstructor`.
- Bind class fields so `self.x` resolves; keep `self` typed `Unknown` until
  Phase F.

*Exit check*: the `class C { function m() { int x = "s"; } }` case above reports
a type mismatch; corpus false positives do not increase.

### Phase C — Bind the remaining symbol kinds *(fixes ~10%)*

- **Function references as values** (#3): when a name misses in the scope stack,
  fall back to the `functions` set before reporting; return `Type::Unknown("function")`.
- **Worker names** (#6): `Stmt::Worker` binds its name into the enclosing scope
  as a future-like symbol so `<- w1` and `wait w1` resolve.
- **Module-qualified symbols**: a `mod:name` reference whose `mod` is an
  imported prefix resolves to `Unknown` instead of erroring.

### Phase D — Fix the type-relation bugs *(fixes ~5%)*

- `can_compare` (#4) — currently a hard-coded list of scalar pairs, so
  `int[] == int[]` is rejected. Replace with: equal types compare; `Unknown` on
  either side compares; arrays/maps compare if their element types do; numeric
  widening stays.
- **`error?` returns** (#7) — `missing-return` already allows implicit nil, but
  `check_stmt`'s `Return` arm does not. Make a nil-able declared return type
  accept a bare `return;`.

### Phase E — Lint-rule traversal parity *(fixes ~14%)*

Four of six rules do not descend into the constructs added during the grammar
work. Measured arms per rule: `camel_case` 4, `constant_case` 0,
`max_function_length` 1, `missing_return` 4, `unused_variables` 12.

- Introduce a shared `visit` helper (`src/linter/visit.rs`) exposing
  `walk_stmts`/`walk_exprs` so a rule opts into node kinds instead of
  re-implementing recursion. `unused_variables.rs` already has the most complete
  walk and is the reference.
- Port every rule onto it, so all reach class/service members, `match` arms,
  `do`/`on fail`, `lock`, `worker`, and query bodies.
- **Parameters (#5)**: stop reporting unused *parameters* by default — a public
  API cannot drop them. Either exclude params, or gate them behind a separate
  `unused-parameters` rule defaulting to `off`.

*Exit check*: `class C { function m() { int BAD_name = 1; } function n() returns int { } }`
reports both camel-case and missing-return.

### Phase F — Deepen type inference *(quality, not false positives)*

Only after A–E are green, since this is where new false positives would come
from. Each step should be measured against the corpus before landing:

1. Record/object field types, so `self.x` and `r.field` resolve.
2. Method-signature lookup for the `lang.*` library (`.length()`, `.push()`,
   `.toString()` …), replacing the current hard-coded table.
3. Union types as a first-class `Type::Union`, so `int|error` narrows under
   `is` and `check`.
4. Named-argument checking against the callee's parameter list (builds on A).

## 3. New lint rules (optional, after A–E)

The rule set is currently six style checks. Constructs the parser now
understands enable genuinely useful rules:

| Rule | Detects |
|---|---|
| `unused-import` | imported module never referenced |
| `unhandled-error` | `error`-returning call whose result is ignored (no `check`/`match`) |
| `unreachable-code` | statements after `return`/`panic`/`fail` |
| `empty-block` | empty `if`/`while`/`match` arm bodies |
| `redundant-check` | `check` on an expression that cannot fail |

## 4. Verification

Per phase:

1. `bash scripts/check.sh` — fmt, clippy `-D warnings`, all tests.
2. `bash scripts/grammar_coverage.sh` — the *"no diagnostics at all"* line is
   the tracked metric; it must rise and never fall.
3. `bash scripts/compare_with_ballerina.sh 60` — **false positives must stay 0**.
   This is the guard rail: it is the reason to fix causes rather than silence
   diagnostics.
4. Add a regression test per root cause in §1, so each fix is pinned.

Tracked metric across the plan:

| Milestone | Files diagnostic-free |
|---|---|
| Today | 99 / 163 (60%) |
| After A | ~130 / 163 (~80%) |
| After A–D | ~145 / 163 (~89%) |
| After A–E | ≥155 / 163 (≥95%) |

## 5. Grammar note: `master` vs `2024R1`

Comparing the two spec revisions:

- `2024R1` — 419 syntactic productions (the released spec; implemented by
  Ballerina 2201.10.0, which this project validates against).
- `master` — 410 syntactic productions; adds **client declarations**
  (`client-decl`, `client-decl-stmt`, `module-client-decl`, `service-uri`),
  which Blazelint does **not** parse. The 13 productions listed as absent are
  largely reorganisations (`collect-clause`, `group-by-clause` remain features).

Recommendation: keep `2024R1` as the comparison baseline, since it is the
released spec matching the compiler used for differential testing. Add client
declarations as a small standalone grammar task — roughly the size of one Phase
A change — rather than folding it into this plan.
