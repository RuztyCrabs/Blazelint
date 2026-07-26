# Blazelint Grammar (BNF)

The grammar Blazelint's parser implements, in the project's original BNF style.

- For a version written in the **official specification's EBNF notation and
  production names** — which is what you want for diffing against the spec — see
  [`EBNF.md`](EBNF.md).
- For measured coverage against the spec, see [`GRAMMAR_COVERAGE.md`](GRAMMAR_COVERAGE.md).
- Reference: [Ballerina Language Specification 2024R1](https://ballerina.io/spec/lang/2024R1/).

> This file was historically titled *"BNF for the selected subset of Ballerina
> Grammar"*. It is no longer a subset: every one of the 419 syntactic
> productions in the 2024R1 specification is accepted, and **391 (93%)** are
> parsed into structured AST nodes. The remaining 28 are accepted as opaque
> blocks without being decomposed — they are marked `(* … *)` below and listed
> individually in [`GRAMMAR_COVERAGE.md`](GRAMMAR_COVERAGE.md) §3.

```bnf
<program> ::= <import_declaration>* <module_level_declaration>*

(* ------------------------------------------------------------------ *)
(* Module level                                                        *)
(* ------------------------------------------------------------------ *)

<import_declaration> ::= "import" <package_name> ["as" <identifier>] ";"
<package_name> ::= IDENTIFIER ("/" IDENTIFIER)* ("." IDENTIFIER)*

<module_level_declaration> ::= <annotation_attachment>* (
                                 <var_declaration>
                               | <const_declaration>
                               | <function_declaration>
                               | <type_definition>
                               | <enum_definition>
                               | <class_definition>
                               )
                             | <configurable_declaration>
                             | <listener_declaration>
                             | <service_declaration>
                             | <annotation_declaration>
                             | <xmlns_declaration>

<qualifier> ::= "public" | "isolated" | "transactional" | "client" | "service" | "distinct"
<annotation_attachment> ::= "@" <identifier> [":" <identifier>] [<map_literal>]

<var_declaration> ::= ["final"] <typed_binding_pattern> ["=" <expression>] ";"
<typed_binding_pattern> ::= ("var" | <type_descriptor>) <binding_pattern>

<const_declaration> ::= "const" [<type_descriptor>] <identifier> "=" <expression> ";"
<configurable_declaration> ::= "configurable" <type_descriptor> <identifier> "=" (<expression> | "?") ";"

<type_definition> ::= "type" IDENTIFIER <type_descriptor> ";"
<enum_definition> ::= "enum" IDENTIFIER "{" [<enum_member> ("," <enum_member>)*] "}"
<enum_member> ::= IDENTIFIER ["=" <expression>]

<class_definition> ::= "class" IDENTIFIER "{" <class_member>* "}"
<class_member> ::= <annotation_attachment>* <member_qualifier>* (
                       <method_declaration>
                     | <type_descriptor> IDENTIFIER ["=" <expression>] ";"
                     | "*" <type_descriptor> ";"
                   )
<member_qualifier> ::= "public" | "private" | "final" | "isolated"
                     | "remote" | "resource" | "readonly" | "transactional"
<method_declaration> ::= "function" <identifier> "(" [<parameters>] ")"
                         ["returns" <type_descriptor>] <function_body>
                       | "function" <identifier> <resource_path> "(" [<parameters>] ")"
                         ["returns" <type_descriptor>] <function_body>
<resource_path> ::= ("/" | "." | IDENTIFIER | "[" <type_descriptor> ["..."] IDENTIFIER "]")*

<listener_declaration> ::= "listener" [<type_descriptor>] IDENTIFIER "=" <expression> ";"
<service_declaration> ::= "service" [<type_descriptor>] [<resource_path>]
                          "on" <expression> "{" <class_member>* "}"
<annotation_declaration> ::= "annotation" ... ";"        (* attach points accepted verbatim *)
<xmlns_declaration> ::= "xmlns" STRING ["as" IDENTIFIER] ";"

<function_declaration> ::= <qualifier>* "function" <identifier>
                           "(" [<parameters>] ")" ["returns" <type_descriptor>] <function_body>
<function_body> ::= <block> | "=>" <expression> ";" | "=" "external" ";"
<parameters> ::= <parameter> ("," <parameter>)*
<parameter> ::= <annotation_attachment>* ["*"] <type_descriptor> ["..."] <identifier>
                ["=" (<expression> | "<" ">")]

(* ------------------------------------------------------------------ *)
(* Type descriptors                                                    *)
(* ------------------------------------------------------------------ *)

<type_descriptor> ::= <type_union>
<type_union> ::= <type_intersection> ("|" <type_intersection>)*
<type_intersection> ::= <type_postfix> ("&" <type_postfix>)*
<type_postfix> ::= <type_primary> (<array_suffix> | "?")*
<array_suffix> ::= "[" [<array_dimension>] "]"
<array_dimension> ::= NUMBER | "*" | IDENTIFIER

<type_primary> ::= <basic_type>
                 | "(" ")"                                  (* nil type *)
                 | "map" "<" <type_descriptor> ">"
                 | <generic_type>
                 | <tuple_type>
                 | <record_type>
                 | <object_type>
                 | <function_type>
                 | <singleton_type>
                 | "distinct" <type_primary>
                 | <type_qualifier>* <type_primary>
                 | <named_type>
<type_qualifier> ::= "isolated" | "client" | "service" | "transactional"
<basic_type> ::= "int" | "string" | "boolean" | "float" | "decimal" | "byte"
               | "anydata" | "json" | "any" | "never" | "readonly" | "handle"
<named_type> ::= IDENTIFIER [":" IDENTIFIER]                (* module- or lang-qualified *)
<generic_type> ::= <named_type> "<" <type_descriptor> ("," <type_descriptor>)* ">"
                   [<key_constraint>]                       (* error<D>, stream<T,C>, table<R> *)
<key_constraint> ::= "key" "(" [IDENTIFIER ("," IDENTIFIER)*] ")"
                   | "key" "<" <type_descriptor> ">"
<tuple_type> ::= "[" [<type_descriptor> ("," <type_descriptor>)*
                      ["," <type_descriptor> "..."]] "]"
<record_type> ::= "record" "{" <record_member>* "}"
                | "record" "{|" <record_member>* "|}"
<record_member> ::= "*" <type_descriptor> ";"                            (* inclusion *)
                  | <type_descriptor> "..." ";"                          (* rest field *)
                  | ["readonly"] <type_descriptor> IDENTIFIER ["?"] ["=" <expression>] ";"
<object_type> ::= "object" "{" ... "}"                      (* body accepted, not decomposed *)
<function_type> ::= "function" ["(" [<parameters>] ")"] ["returns" <type_descriptor>]
<singleton_type> ::= ["-"] (NUMBER | STRING | "true" | "false")

(* ------------------------------------------------------------------ *)
(* Binding patterns                                                    *)
(* ------------------------------------------------------------------ *)

<binding_pattern> ::= <identifier> | "_"
                    | "[" [<binding_pattern> ("," <binding_pattern>)*] ["," <rest_binding>] "]"
                    | "{" [<field_binding> ("," <field_binding>)*] ["," <rest_binding>] "}"
                    | <error_binding>
<field_binding> ::= <identifier> [":" <binding_pattern>]    (* shorthand `{x}` == `{x: x}` *)
<rest_binding> ::= "..." <identifier>
<error_binding> ::= [<named_type>] "error" "(" [<error_arg_binding>
                    ("," <error_arg_binding>)*] ")"
<error_arg_binding> ::= ["var"] <identifier> ["=" ["var"] <identifier>] | <binding_pattern>

(* ------------------------------------------------------------------ *)
(* Statements                                                          *)
(* ------------------------------------------------------------------ *)

<block> ::= "{" <statement>* "}"

<statement> ::= <var_declaration>
              | <destructuring_declaration>
              | <if_statement>       | <while_statement>   | <foreach_statement>
              | <match_statement>    | <do_statement>      | <lock_statement>
              | <fork_statement>     | <transaction_statement> | <retry_statement>
              | <worker_declaration> | <return_statement>  | <panic_statement>
              | <fail_statement>     | <rollback_statement>
              | <break_statement>    | <continue_statement>
              | <xmlns_declaration>  | <expression_statement> | <block>

<destructuring_declaration> ::= ["var"] [<type_descriptor>] <binding_pattern> "=" <expression> ";"

<if_statement> ::= "if" <expression> <block>
                   ["else" (<if_statement> | <block>)]      (* parentheses optional *)
<while_statement> ::= "while" <expression> <block>
<foreach_statement> ::= "foreach" ("var" | <type_descriptor>) <binding_pattern>
                        "in" <expression> <block>
<break_statement> ::= "break" ";"
<continue_statement> ::= "continue" ";"
<return_statement> ::= "return" [<expression>] ";"
<panic_statement> ::= "panic" <expression> ";"
<fail_statement> ::= "fail" <expression> ";"
<rollback_statement> ::= "rollback" [<expression>] ";"

<match_statement> ::= "match" <expression> "{" <match_arm>* "}"
<match_arm> ::= <match_pattern> ("|" <match_pattern>)* ["if" <expression>] "=>" <block>
<match_pattern> ::= "_"
                  | ["-"] (NUMBER | STRING | "true" | "false" | "(" ")")
                  | ["var"] <identifier>
                  | ["var"] "[" [<match_pattern> ("," <match_pattern>)*] "]"
                  | ["var"] "{" [<field_match> ("," <field_match>)*] ["," <rest_match>] "}"
                  | [<named_type>] "error" "(" [<error_arg_match> ("," <error_arg_match>)*] ")"
<field_match> ::= <identifier> [":" <match_pattern>]
<rest_match> ::= "..." ["var"] <identifier>
<error_arg_match> ::= [<identifier> "="] <match_pattern>

<do_statement> ::= "do" <block> [<on_fail_clause>]
<on_fail_clause> ::= "on" "fail" [("var" | <type_descriptor>)] <identifier> <block>
<lock_statement> ::= "lock" <block> [<on_fail_clause>]
<fork_statement> ::= "fork" "{" ... "}"                     (* body accepted, not decomposed *)
<worker_declaration> ::= "worker" <identifier> ["returns" <type_descriptor>] <block>
<transaction_statement> ::= "transaction" <block>
<retry_statement> ::= "retry" ["<" <type_descriptor> ">"] ["(" [<expression>] ")"]
                      ["transaction"] <block>
<expression_statement> ::= <expression> ";"

(* ------------------------------------------------------------------ *)
(* Expressions (loosest binding first)                                 *)
(* ------------------------------------------------------------------ *)

<expression> ::= <assignment>
<assignment> ::= <lvalue> <assignment_op> <assignment> | <ternary>
<lvalue> ::= <identifier> | <postfix>                       (* variable, field, or index *)
<assignment_op> ::= "=" | "+=" | "-="

<ternary> ::= <range> ("?" <expression> ":" <ternary>)?
            | <range> "?:" <range>
<range> ::= <logic_or> (("..." | "..<") <logic_or>)?
<logic_or> ::= <logic_and> ("||" <logic_and>)*
<logic_and> ::= <equality> ("&&" <equality>)*
<equality> ::= <comparison> (("==" | "!=" | "===" | "!==") <comparison>)*
<comparison> ::= <shift> ((">" | ">=" | "<" | "<=") <shift> | "is" <type_descriptor>)*
<shift> ::= <additive> (("<<" | ">>" | ">>>") <additive>)*
<additive> ::= <bitwise> (("+" | "-") <bitwise>)*
<bitwise> ::= <multiplicative> (("&" | "|" | "^") <multiplicative>)*
<multiplicative> ::= <unary> (("*" | "/" | "%") <unary>)*

<unary> ::= ("!" | "-" | "~" | "+") <unary>
          | ("check" | "checkpanic" | "trap" | "wait") <unary>
          | "typeof" <unary>
          | "flush" [<identifier>]
          | "<-" <identifier>                               (* worker receive *)
          | <let_expression>
          | <postfix>
<let_expression> ::= "let" <let_binding> ("," <let_binding>)* "in" <expression>
<let_binding> ::= ["final"] ("var" | <type_descriptor>) IDENTIFIER "=" <expression>

<postfix> ::= <primary> <postfix_op>*
<postfix_op> ::= "(" [<call_arguments>] ")"                        (* call *)
               | ["?"] "." <identifier> ["(" [<call_arguments>] ")"] (* field / method *)
               | "." "@" <identifier>                              (* annotation access *)
               | "." "<" <xml_name_pattern> ">"                    (* xml filter *)
               | "/" ("*" | "<" <xml_name_pattern> ">")            (* xml children *)
               | "/" "**" "/" "<" <xml_name_pattern> ">"           (* xml descendants *)
               | "[" <expression> ("," <expression>)* "]"          (* member / multi-key *)
               | "->" (<identifier> | <resource_access_path>) ["(" [<call_arguments>] ")"]
               | "->>" <identifier>                                (* synchronous send *)
               | ":" <identifier> ["(" [<call_arguments>] ")"]     (* qualified reference *)
<resource_access_path> ::= ("/" (<identifier> | "[" ["..."] <expression> "]"))*
                           ["." <identifier>]
<xml_name_pattern> ::= <xml_atomic_name> ("|" <xml_atomic_name>)*
<xml_atomic_name> ::= "*" | IDENTIFIER [":" (IDENTIFIER | "*")]

<call_arguments> ::= <call_argument> ("," <call_argument>)*
<call_argument> ::= ["..."] <expression>                    (* rest argument *)
                  | <identifier> "=" <expression>           (* named argument *)

<primary> ::= <number_literal> | <string_literal> | <template>
            | "true" | "false" | "()"
            | <identifier> | "self" | "commit" | "transactional"
            | <array_literal> | <map_literal> | <table_constructor>
            | <object_constructor> | <new_expression> | <error_constructor>
            | <anonymous_function> | <arrow_function>
            | <query_expression> | <start_action> | <wait_action>
            | "(" <expression> ")" | <cast_expression>

<number_literal> ::= NUMBER | HEX_NUMBER                    (* `d`/`f` suffix accepted *)
<string_literal> ::= DOUBLE_QUOTE_STRING
<template> ::= [<template_tag>] "`" <template_part>* "`"
<template_tag> ::= "string" | "xml" | "re" | "base16" | "base64" | IDENTIFIER
<template_part> ::= STRING_CHAR | "${" <expression> "}"

<array_literal> ::= "[" [<list_member> ("," <list_member>)*] "]"
<list_member> ::= ["..."] <expression>
<map_literal> ::= "{" [<map_entry> ("," <map_entry>)*] "}"
<map_entry> ::= (IDENTIFIER | STRING) [":" <expression>]    (* shorthand `{x}` *)
              | "[" <expression> "]" ":" <expression>       (* computed key *)
              | "..." <expression>                          (* spread *)
<table_constructor> ::= "table" [<key_constraint>] "[" [<expression> ("," <expression>)*] "]"
<object_constructor> ::= <type_qualifier>* "object" [":" <named_type>] "{" <class_member>* "}"
<new_expression> ::= "new" [<type_descriptor>] ["(" [<call_arguments>] ")"]
<error_constructor> ::= "error" [<named_type>] "(" [<call_arguments>] ")"
<cast_expression> ::= "<" <type_descriptor> ">" <unary>
<start_action> ::= "start" <postfix>
<wait_action> ::= "wait" (<expression> ("|" <expression>)* | <map_literal>)

<anonymous_function> ::= ["isolated"] "function" "(" [<parameters>] ")"
                         ["returns" <type_descriptor>] (<block> | "=>" <expression>)
<arrow_function> ::= <identifier> "=>" <expression>
                   | "(" [<arrow_param> ("," <arrow_param>)*] ")" "=>" <expression>
<arrow_param> ::= [<type_descriptor>] IDENTIFIER

(* ------------------------------------------------------------------ *)
(* Query expressions                                                   *)
(* ------------------------------------------------------------------ *)

<query_expression> ::= [<query_construct_type>] <from_clause> <query_clause>*
                       (<select_clause> | <collect_clause> | <do_clause>)
                       [<on_conflict_clause>]
<query_construct_type> ::= "map" | "table" | "stream"
<from_clause> ::= "from" ("var" | [<type_descriptor>]) <binding_pattern> "in" <expression>
<query_clause> ::= <from_clause>
                 | "where" <expression>
                 | "let" <let_binding> ("," <let_binding>)*      (* no trailing "in" *)
                 | ["outer"] "join" ("var" | [<type_descriptor>]) <binding_pattern>
                   "in" <expression> "on" <expression> "equals" <expression>
                 | "order" "by" <order_key> ("," <order_key>)*
                 | "limit" <expression>
                 | "group" "by" <expression>
<order_key> ::= <expression> ["ascending" | "descending"]
<select_clause> ::= "select" <expression>
<collect_clause> ::= "collect" <expression>
<do_clause> ::= "do" <block>
<on_conflict_clause> ::= "on" "conflict" <expression>

(* ------------------------------------------------------------------ *)
(* Lexical                                                             *)
(* ------------------------------------------------------------------ *)

<identifier> ::= IDENTIFIER | "'" IDENTIFIER                (* quoted: keywords as names *)
<comment> ::= "//" <any_char_but_newline>* NEWLINE
<documentation> ::= "#" <any_char_but_newline>* NEWLINE     (* skipped like a comment *)
```

> **Ballerina has no block comments.** `/* … */` is not a comment form — the
> official compiler rejects `/*` as an invalid token, and treating it as one
> would swallow the XML all-children navigation step `x/*`.
