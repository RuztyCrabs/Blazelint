# Plan: Parity with `bal scan` Rules

**Goal** — move from 1 of 27 official scan rules to 8, by implementing the rules
our AST already supports.

**Why these** — the parser can already see `checkpanic`, ranges, assignments, and
`isolated`. Nothing reads them. These four rule groups need one structural
change and no new analysis capability.

**Reference** — [Ballerina scan rules](https://ballerina.io/learn/scan-rules/):
12 language rules (`ballerina:1`–`:12`) and 15 library rules.

---

## 1. Current parity

| | Count |
|---|---|
| Official scan rules | 27 (12 language + 15 library) |
| Implemented | **1** — `ballerina:2` unused function parameter |
| After this plan | **8** (30%) |

Our other six rules (`camel-case`, `constant-case`, `line-length`,
`max-function-length`, `missing-return`, `unused-variables`) are not in the scan
ruleset, and our 25 semantic checks overlap `bal build`, not `bal scan`. They
stay — they are useful — but they do not count toward parity.

## 2. Scope: four rule groups, seven new rules

| Rule | ID | Kind | Detects |
|---|---|---|---|
| `avoid-checkpanic` | `ballerina:1` | Code Smell | `checkpanic` used instead of `check` |
| `self-assignment` | `ballerina:10` | Code Smell | `x = x`, `self.f = self.f` |
| `invalid-range` | `ballerina:12` | Code Smell | `9...0` — a range that cannot iterate |
| `isolated-public-function` | `ballerina:3` | Code Smell | public function not marked `isolated` |
| `isolated-public-method` | `ballerina:4` | Code Smell | public method not marked `isolated` |
| `isolated-public-class` | `ballerina:5` | Code Smell | public class not marked `isolated` |

Plus `ballerina:2` (already shipped as `unused-parameters`) — see §5 on its
default.

Deliberately **excluded** from this batch, and why:

- `ballerina:7` `:8` `:9` (always true/false/same) — needs constant folding and
  knowledge of `int:MAX_VALUE`-style bounds.
- `ballerina:11` (unused private fields) — needs `self.x` read/write tracking.
- `ballerina:6` (isolated public object) — object *type* bodies are one of the 28
  opaque productions; they would have to be parsed into members first.
- All 15 library rules — roughly half are pattern matches on a qualified call
  plus a literal argument and are a good follow-up batch; the other half
  (path injection, SSRF, open redirect, command injection, IV reuse) need taint
  analysis we do not have.

## 3. The one structural change

`Stmt::Function` records `is_public` but **not** the other qualifiers — the
parser takes them and throws them away:

```rust
fn function(&mut self, is_public: bool, _qualifiers: Vec<String>) -> ParseResult<Stmt>
```

`Stmt::ClassDef` already keeps its `qualifiers: Vec<String>`, so classes need
nothing. Two changes:

1. **`src/ast.rs`** — add `qualifiers: Vec<String>` to `Stmt::Function`.
2. **`src/parser.rs`** — stop discarding them in `function()`; in
   `class_member()`, collect the qualifier loop's words instead of skipping them
   (it currently just `continue`s past `public`/`isolated`/`private`/…), and
   record whether the member was `public`.

This is what unblocks `ballerina:3` and `:4`.

## 4. Implementation

Each rule is a unit struct implementing `LintRule`, exported from
`src/linter/rules/mod.rs` and registered in `src/lib.rs` beside the existing
seven. Reuse `MissingReturnRule::check_functions` as the traversal model — it
already reaches class and service members.

### `avoid-checkpanic` (`ballerina:1`)
Walk expressions for `Expr::Check { keyword: "checkpanic", .. }`. The keyword is
already retained on the node. Report at the expression span.

*Message*: "Avoid `checkpanic`; use `check` and handle the error, or return it."

### `self-assignment` (`ballerina:10`)
Two shapes:
- `Expr::Assign { name, value }` where `value` is `Expr::Variable` with the same
  name.
- `Expr::MemberAssign { target, value }` where both are `FieldAccess` chains
  that render identically (`self.f = self.f`).

Add a small `same_lvalue(&Expr, &Expr) -> bool` helper comparing shape, not
spans.

### `invalid-range` (`ballerina:12`)
`Expr::Range { start, end }` where both are integer literals and `start > end`
(for `...`; `..<` is empty when `start >= end`). Only literal endpoints — do not
guess at variables.

*Note*: the AST does not currently distinguish `...` from `..<` — both parse to
`Expr::Range`. Add an `inclusive: bool` field, since `1..<1` is a legitimate
empty range while `1...0` is the reported mistake.

### `isolated-public-*` (`ballerina:3` `:4` `:5`)
One rule struct per kind so severities can be configured separately:
- **function**: `Stmt::Function { is_public: true, .. }` at module level whose
  `qualifiers` lack `isolated`.
- **method**: same, for members of a `ClassDef`/`ServiceDecl` that are `public`.
- **class**: `Stmt::ClassDef { is_public: true, qualifiers }` lacking `isolated`.

All three default to **`off`**. They are advisory concurrency guidance, and on a
codebase that does not use concurrency they would fire on nearly every public
declaration. Users opt in.

## 5. Configuration and defaults

Add to `Config::default()`:

| Rule | Default | Rationale |
|---|---|---|
| `avoid-checkpanic` | `warn` | Real defect risk; matches scan's Code Smell |
| `self-assignment` | `warn` | Almost always a bug |
| `invalid-range` | `warn` | Almost always a bug |
| `isolated-public-function` | `off` | Advisory; noisy outside concurrent code |
| `isolated-public-method` | `off` | ” |
| `isolated-public-class` | `off` | ” |
| `unused-parameters` | `off` → **`warn`** | Reconsider: `bal scan` ships this on as `ballerina:2`. Aligning improves parity honestly, at the cost of noise on callback signatures. |

Note the `unused-parameters` question is a genuine trade-off, not an oversight:
we turned it off last session precisely because it fired 27 times on the example
corpus, nearly all on signatures that cannot drop a parameter. Recommend keeping
it `off` by default and documenting the divergence, rather than importing the
noise to match a number.

## 6. Verification

Per rule:

1. A unit test with a noncompliant and a compliant example, taken from the
   official rule page so behaviour matches the documented intent.
2. `bash scripts/check.sh` green.
3. `bash scripts/grammar_coverage.sh` — the diagnostic-free count will **fall**,
   because these rules find real issues in the corpus. That is expected; confirm
   each new report is a true positive by inspection.
4. `bash scripts/compare_with_ballerina.sh` — must stay at **0 false
   positives**. These are lint opinions, not compile errors, so the official
   compiler accepting a file is not evidence a rule is wrong; check the rule's
   own examples instead.

Expected corpus impact, to be confirmed rather than assumed: `checkpanic` and
`invalid-range` should fire rarely; the `isolated-*` rules would fire on most
public declarations, which is why they ship off.

## 7. After this batch

The next highest-value step is the ~7 library rules that are pattern matches on
a qualified call plus a literal argument — `crypto:encryptAesEcb`,
`hashBcrypt(pw, <10)`, `jwt:NONE`, `verifyHostName: false`,
`allowOrigins: ["*"]`, `resource function default`, and a `configurable`
variable passed to `log:print*`. We already track imports and parse
`configurable`, so these need no new analysis — only a table of module,
function, and argument patterns. That would take parity to roughly 15/27 (56%).

Taint analysis for the remaining injection rules is a larger, separate project.
