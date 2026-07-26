# Blazelint Grammar (EBNF)

The grammar Blazelint's parser actually implements, written in the **same EBNF
notation and production names as the official Ballerina specification** so the
two can be diffed directly.

- Reference: [Ballerina Language Specification 2024R1](https://ballerina.io/spec/lang/2024R1/)
- Notation: `:=` defines a production, `|` alternates, `[x]` is optional,
  `(x)*` repeats zero or more times, `(x)+` one or more, `"x"` is a literal token.
- Coverage of this grammar against the official spec: see [`GRAMMAR_COVERAGE.md`](GRAMMAR_COVERAGE.md).

A prose-oriented version of the same grammar is in [`BNF.md`](BNF.md).

> **Deliberate deviations** are marked `(* … *)`. They exist because Blazelint is
> a single-file linter, not a compiler: some sub-grammars are accepted but not
> decomposed, since no lint rule inspects their internals.

---

## 1. Module structure

```ebnf
module-part := [import-decl]* [other-decl]*

import-decl := "import" [org-name "/"] module-name ["as" import-prefix] ";"
org-name := import-identifier
module-name := import-identifier ("." import-identifier)*
import-prefix := identifier | "_"

other-decl :=
    function-defn | module-type-defn | module-class-defn
  | module-var-decl | module-const-decl | module-enum-decl
  | module-xmlns-decl | listener-decl | service-decl | annotation-decl

metadata := [DocumentationString] [annots]
annots := annotation*
annotation := "@" annot-tag-reference [mapping-constructor-expr]
annot-tag-reference := identifier [":" identifier]
```

## 2. Declarations

```ebnf
function-defn := metadata function-quals "function" identifier function-signature function-defn-body
function-quals := ("public" | "isolated" | "transactional")*
function-signature := "(" [param-list] ")" [return-type-descriptor]
return-type-descriptor := "returns" [annots] type-descriptor

function-defn-body := block-function-body | expr-function-body | external-function-body
block-function-body := "{" statement* "}"
expr-function-body := "=>" expression ";"
external-function-body := "=" "external" ";"

param-list := param ("," param)*
param := [annots] ["*"] type-descriptor ["..."] param-name ["=" default-value]
default-value := expression | "<" ">"        (* inferred-typedesc-default *)
param-name := identifier

module-type-defn := metadata ["public"] "type" identifier type-descriptor ";"

module-class-defn := metadata class-type-quals "class" identifier "{" class-member* "}"
class-type-quals := ("public" | "distinct" | "readonly" | "isolated" | "client" | "service")*
class-member := metadata object-member-qual*
                ( method-defn | object-field | object-type-inclusion )
object-member-qual := "public" | "private" | "final" | "isolated"
                    | "remote" | "resource" | "readonly" | "transactional"
object-field := type-descriptor identifier ["=" expression] ";"
object-type-inclusion := "*" type-descriptor ";"
method-defn := "function" method-name function-signature function-defn-body
             | "function" resource-accessor resource-path function-signature function-defn-body
resource-accessor := identifier
resource-path := ( "/" | "." | identifier | "[" type-descriptor ["..."] identifier "]" )*

module-var-decl := metadata module-var-quals typed-binding-pattern ["=" expression] ";"
module-var-quals := ("public" | "final" | "isolated" | "configurable")*
                    (* `configurable T x = ?;` accepted; `?` marks a required value *)

module-const-decl := metadata ["public"] "const" [type-descriptor] identifier "=" const-expr ";"
module-enum-decl := metadata ["public"] "enum" identifier "{" [enum-member ("," enum-member)*] "}"
enum-member := metadata identifier ["=" const-expr]
module-xmlns-decl := "xmlns" string-literal ["as" identifier] ";"

listener-decl := metadata ["public"] "listener" [type-descriptor] identifier "=" expression ";"
service-decl := metadata ["isolated"] "service" [type-descriptor] [absolute-resource-path]
                "on" expression-list "{" class-member* "}"
absolute-resource-path := ("/" identifier)* | "/"

annotation-decl := metadata ["public"] "annotation" ... ";"
                   (* attach-point list accepted without decomposition *)
```

## 3. Type descriptors

```ebnf
type-descriptor := union-type-descriptor
union-type-descriptor := intersection-type-descriptor ("|" intersection-type-descriptor)*
intersection-type-descriptor := type-postfix ("&" type-postfix)*
type-postfix := type-primary (array-suffix | "?")*
array-suffix := "[" [array-length] "]"
array-length := int-literal | "*" | constant-reference-expr

type-primary :=
    simple-type-descriptor | string-type-descriptor | xml-type-descriptor
  | map-type-descriptor | record-type-descriptor | object-type-descriptor
  | tuple-type-descriptor | function-type-descriptor | generic-type-descriptor
  | singleton-type-descriptor | distinct-type-descriptor | type-reference
  | nil-type-descriptor | type-qualifier* type-primary

simple-type-descriptor := "int" | "float" | "decimal" | "boolean" | "byte"
                        | "anydata" | "json" | "any" | "never" | "readonly" | "handle"
nil-type-descriptor := "(" ")"
type-qualifier := "isolated" | "client" | "service" | "transactional"

map-type-descriptor := "map" "<" type-descriptor ">"
generic-type-descriptor := type-reference "<" type-descriptor ("," type-descriptor)* ">"
                           [key-constraint]
                           (* covers error<D>, future<T>, stream<T,C>, typedesc<T>, table<R> *)
key-constraint := "key" "(" [identifier ("," identifier)*] ")"
                | "key" "<" type-descriptor ">"

tuple-type-descriptor := "[" [type-descriptor ("," type-descriptor)*
                              [["," ] type-descriptor "..."]] "]"

record-type-descriptor := "record" "{" record-member* "}"
                        | "record" "{|" record-member* "|}"
record-member := "*" type-descriptor ";"                              (* inclusion *)
               | type-descriptor "..." ";"                            (* rest field *)
               | ["readonly"] type-descriptor field-name ["?"] ["=" expression] ";"
field-name := identifier

object-type-descriptor := "object" "{" ... "}"
                          (* body accepted as a balanced block; members not retained *)

function-type-descriptor := "function" ["(" [param-list] ")"] [return-type-descriptor]

singleton-type-descriptor := ["-"] (int-literal | floating-point-literal
                                   | string-literal | boolean-literal)
distinct-type-descriptor := "distinct" type-primary
type-reference := identifier [":" identifier]
                | builtin-module ":" identifier       (* e.g. int:Signed32, object:RawTemplate *)
builtin-module := "int" | "string" | "float" | "decimal" | "boolean" | "map"
                | "object" | "function" | "type" | "transaction" | "xml" | "error"
```

## 4. Binding patterns

```ebnf
typed-binding-pattern := ("var" | type-descriptor) binding-pattern
binding-pattern := capture-binding-pattern | wildcard-binding-pattern
                 | list-binding-pattern | mapping-binding-pattern | error-binding-pattern
capture-binding-pattern := identifier
wildcard-binding-pattern := "_"
list-binding-pattern := "[" [binding-pattern ("," binding-pattern)*] ["," rest-binding-pattern] "]"
mapping-binding-pattern := "{" [field-binding-pattern ("," field-binding-pattern)*]
                           ["," rest-binding-pattern] "}"
field-binding-pattern := field-name [":" binding-pattern]      (* shorthand `{x}` == `{x: x}` *)
rest-binding-pattern := "..." identifier
error-binding-pattern := [type-reference] "error" "(" [error-arg-binding-pattern
                         ("," error-arg-binding-pattern)*] ")"
error-arg-binding-pattern := ["var"] identifier ["=" ["var"] identifier] | binding-pattern
```

## 5. Statements

```ebnf
statement :=
    local-var-decl-stmt | destructuring-assignment-stmt | assignment-stmt
  | compound-assignment-stmt | if-else-stmt | while-stmt | foreach-stmt
  | match-stmt | do-stmt | lock-stmt | fork-stmt | transaction-stmt
  | retry-stmt | rollback-stmt | named-worker-decl | return-stmt
  | panic-stmt | fail-stmt | break-stmt | continue-stmt
  | xmlns-decl-stmt | statement-block | action-or-expr-stmt

statement-block := "{" statement* "}"

local-var-decl-stmt := ["final"] typed-binding-pattern ["=" expression] ";"
destructuring-assignment-stmt := ["var"] [type-descriptor] binding-pattern "=" expression ";"
assignment-stmt := lvexpr "=" expression ";"
compound-assignment-stmt := lvexpr CompoundAssignmentOperator expression ";"
lvexpr := variable-reference | field-access-expr | member-access-expr
CompoundAssignmentOperator := "+=" | "-="

if-else-stmt := "if" expression statement-block
                ["else" (if-else-stmt | statement-block)]
                (* parentheses around the condition are optional, per the spec *)
while-stmt := "while" expression statement-block
foreach-stmt := "foreach" ("var" | type-descriptor) binding-pattern "in" expression statement-block
break-stmt := "break" ";"
continue-stmt := "continue" ";"
return-stmt := "return" [expression] ";"
panic-stmt := "panic" expression ";"
fail-stmt := "fail" expression ";"

match-stmt := "match" expression "{" match-clause* "}"
match-clause := match-pattern ("|" match-pattern)* [match-guard] "=>" statement-block
match-guard := "if" expression
match-pattern := wildcard-match-pattern | const-pattern | capture-pattern
               | list-match-pattern | mapping-match-pattern | error-match-pattern
wildcard-match-pattern := "_"
const-pattern := ["-"] (int-literal | floating-point-literal | string-literal
                       | boolean-literal | nil-literal)
capture-pattern := ["var"] identifier
list-match-pattern := ["var"] "[" [match-pattern ("," match-pattern)*] "]"
mapping-match-pattern := ["var"] "{" [field-match-pattern ("," field-match-pattern)*]
                         ["," rest-match-pattern] "}"
field-match-pattern := field-name [":" match-pattern]
rest-match-pattern := "..." ["var"] identifier
error-match-pattern := [type-reference] "error" "(" [error-arg-match-pattern
                       ("," error-arg-match-pattern)*] ")"
error-arg-match-pattern := [field-name "="] match-pattern

do-stmt := "do" statement-block [on-fail-clause]
on-fail-clause := "on" "fail" [("var" | type-descriptor)] identifier statement-block
lock-stmt := "lock" statement-block [on-fail-clause]
fork-stmt := "fork" "{" ... "}"          (* body accepted as a balanced block *)
named-worker-decl := "worker" identifier [return-type-descriptor] statement-block
transaction-stmt := "transaction" statement-block
retry-stmt := "retry" ["<" type-descriptor ">"] ["(" [expression] ")"]
              ["transaction"] statement-block
rollback-stmt := "rollback" [expression] ";"
xmlns-decl-stmt := "xmlns" string-literal ["as" identifier] ";"
action-or-expr-stmt := action-or-expr ";"
```

## 6. Expressions

Listed loosest-binding first; each level falls through to the next.

```ebnf
expression := assignment-expr
assignment-expr := lvexpr ("=" | "+=" | "-=") assignment-expr | conditional-expr
conditional-expr := range-expr ["?" expression ":" conditional-expr]   (* ternary *)
                  | range-expr "?:" range-expr                          (* elvis *)
range-expr := logical-or-expr [("..." | "..<") logical-or-expr]
logical-or-expr := logical-and-expr ("||" logical-and-expr)*
logical-and-expr := equality-expr ("&&" equality-expr)*
equality-expr := relational-expr (("==" | "!=" | "===" | "!==") relational-expr)*
relational-expr := shift-expr (( "<" | "<=" | ">" | ">=" ) shift-expr | is-expr)*
is-expr := "is" type-descriptor
shift-expr := additive-expr (("<<" | ">>" | ">>>") additive-expr)*
additive-expr := binary-bitwise-expr (("+" | "-") binary-bitwise-expr)*
binary-bitwise-expr := multiplicative-expr (("&" | "|" | "^") multiplicative-expr)*
multiplicative-expr := unary-expr (("*" | "/" | "%") unary-expr)*

unary-expr := ("!" | "-" | "+" | "~") unary-expr
            | checking-keyword unary-expr
            | "typeof" unary-expr
            | "flush" [peer-worker]
            | "<-" peer-worker                         (* receive action *)
            | let-expr
            | postfix-expr
checking-keyword := "check" | "checkpanic" | "trap" | "wait"
let-expr := "let" let-var-decl ("," let-var-decl)* "in" expression
let-var-decl := ["final"] ("var" | type-descriptor) identifier "=" expression

postfix-expr := primary-expr postfix-op*
postfix-op := "(" [arg-list] ")"                        (* call *)
            | ["?"] "." identifier ["(" [arg-list] ")"] (* field access / method call *)
            | "." "@" identifier                        (* annot / attribute access *)
            | "." "<" xml-name-pattern ">"              (* xml filter *)
            | "/" ("*" | "<" xml-name-pattern ">")      (* xml children / all-children *)
            | "/" "**" "/" "<" xml-name-pattern ">"     (* xml descendants *)
            | "[" expression ("," expression)* "]"      (* member / multi-key access *)
            | "->" (identifier | resource-access-path) ["(" [arg-list] ")"]
            | "->>" peer-worker                         (* synchronous send *)
            | ":" identifier ["(" [arg-list] ")"]       (* qualified reference / call *)
resource-access-path := ("/" (identifier | "[" ["..."] expression "]"))* ["." identifier]
xml-name-pattern := xml-atomic-name ("|" xml-atomic-name)*
xml-atomic-name := "*" | identifier [":" (identifier | "*")]
peer-worker := identifier | "function"

arg-list := arg ("," arg)*
arg := ["..."] expression | identifier "=" expression    (* rest / named argument *)

primary-expr :=
    literal | template-expr | structural-constructor-expr
  | object-constructor-expr | new-expr | error-constructor-expr
  | anonymous-function-expr | let-expr | query-expr | table-constructor-expr
  | type-cast-expr | start-action | wait-action | variable-reference
  | "(" expression ")" | nil-literal

literal := int-literal | floating-point-literal | string-literal
         | boolean-literal | nil-literal
int-literal := DecimalNumber | HexIntLiteral            (* `0x1F`; `d`/`f` suffix accepted *)
nil-literal := "(" ")"

template-expr := [data-tag] BacktickString
data-tag := "string" | "xml" | "re" | "base16" | "base64" | identifier
            (* interpolations `${expr}` are parsed as sub-expressions *)

structural-constructor-expr := list-constructor-expr | mapping-constructor-expr
list-constructor-expr := "[" [list-member ("," list-member)*] "]"
list-member := ["..."] expression
mapping-constructor-expr := "{" [field ("," field)*] "}"
field := specific-field | computed-name-field | spread-field
specific-field := (identifier | string-literal) [":" expression]   (* shorthand `{x}` *)
computed-name-field := "[" expression "]" ":" expression
spread-field := "..." expression

table-constructor-expr := "table" [key-specifier] "[" [expression ("," expression)*] "]"
object-constructor-expr := object-quals "object" [":" type-reference] "{" class-member* "}"
object-quals := ("isolated" | "client" | "service")*
new-expr := "new" [type-descriptor] ["(" [arg-list] ")"]
error-constructor-expr := "error" [type-reference] "(" [arg-list] ")"

anonymous-function-expr := explicit-anonymous-function-expr | infer-anonymous-function-expr
explicit-anonymous-function-expr := ["isolated"] "function" "(" [param-list] ")"
                                    [return-type-descriptor]
                                    (block-function-body | "=>" expression)
infer-anonymous-function-expr := infer-param-list "=>" expression
infer-param-list := identifier | "(" [[type-descriptor] identifier
                                      ("," [type-descriptor] identifier)*] ")"

type-cast-expr := "<" type-descriptor ">" unary-expr
start-action := "start" postfix-expr
wait-action := "wait" (expression | "{" wait-field ("," wait-field)* "}")
             | "wait" expression ("|" expression)*
wait-field := identifier [":" expression]
variable-reference := identifier | "self" | "commit" | "transactional"
                    | builtin-module ":" identifier
```

## 7. Query expressions

```ebnf
query-expr := [query-construct-type] query-pipeline (select-clause | collect-clause | do-clause)
              [on-conflict-clause]
query-construct-type := "map" | "table" | "stream"
query-pipeline := from-clause intermediate-clause*
intermediate-clause := from-clause | where-clause | let-clause | join-clause
                     | order-by-clause | limit-clause | group-by-clause

from-clause := "from" ("var" | [type-descriptor]) binding-pattern "in" expression
where-clause := "where" expression
let-clause := "let" let-var-decl ("," let-var-decl)*         (* no trailing "in" *)
join-clause := ["outer"] "join" ("var" | [type-descriptor]) binding-pattern
               "in" expression "on" expression "equals" expression
order-by-clause := "order" "by" order-key ("," order-key)*
order-key := expression ["ascending" | "descending"]
limit-clause := "limit" expression
group-by-clause := "group" "by" expression
select-clause := "select" expression
collect-clause := "collect" expression
do-clause := "do" statement-block                             (* query action *)
on-conflict-clause := "on" "conflict" expression
```

## 8. Lexical structure

```ebnf
identifier := UnquotedIdentifier | QuotedIdentifier
UnquotedIdentifier := IdentifierInitialChar IdentifierFollowingChar*
QuotedIdentifier := "'" IdentifierFollowingChar*         (* lets keywords be identifiers *)

Comment := "//" AnyCharButNewline* NewLine
DocumentationLine := "#" AnyCharButNewline* NewLine       (* skipped like a comment *)
```

> Ballerina has **no** `/* … */` block comment — the official compiler rejects
> `/*` as an invalid token, and treating it as a comment would swallow the XML
> all-children step `x/*`.

Numeric, string, byte-array (`base16`/`base64`), and backtick-template lexemes
follow the spec's lexical productions. The regular-expression sub-grammar inside
`` re `…` `` templates is captured verbatim rather than decomposed — see
[`GRAMMAR_COVERAGE.md`](GRAMMAR_COVERAGE.md) §3.
