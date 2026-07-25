### BNF for the selected subset of Ballerina Grammar

```bnf
<program> ::= <import_declaration>* <module_level_declaration>*

<import_declaration> ::= "import" <package_name> ";"
<package_name> ::= IDENTIFIER ("/" IDENTIFIER)*

<module_level_declaration> ::= <qualifier>* (
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
<qualifier> ::= "public" | "isolated"

<var_declaration> ::= ["final"] <typed_binding_pattern> "=" <expression> ";"
                    | ["final"] <type_descriptor> <identifier> ";"
<typed_binding_pattern> ::= "var" <identifier>
                          | <type_descriptor> <identifier>

<const_declaration> ::= "const" <identifier> "=" <expression> ";"

<type_definition> ::= "type" IDENTIFIER <type_descriptor> ";"
<enum_definition> ::= "enum" IDENTIFIER "{" [<enum_member> ("," <enum_member>)*] "}"
<enum_member> ::= IDENTIFIER ["=" <expression>]
<class_definition> ::= "class" IDENTIFIER "{" <class_member>* "}"
<class_member> ::= <member_qualifier>* (
                       <function_declaration>
                     | <type_descriptor> IDENTIFIER ["=" <expression>] ";"
                     | "*" <type_descriptor> ";"
                   )
<member_qualifier> ::= "public" | "private" | "final" | "isolated"
                     | "remote" | "resource" | "readonly"
<configurable_declaration> ::= "configurable" <type_descriptor> IDENTIFIER "=" (<expression> | "?") ";"
<listener_declaration> ::= "listener" <type_descriptor> IDENTIFIER "=" <expression> ";"
<service_declaration> ::= "service" ... "on" <expression> <block>   (* header parsed leniently *)
<annotation_declaration> ::= "annotation" ... ";"                    (* parsed leniently *)
<xmlns_declaration> ::= "xmlns" STRING ["as" IDENTIFIER] ";"

<type_descriptor> ::= <type_union>
<type_union> ::= <type_intersection> ("|" <type_intersection>)*
<type_intersection> ::= <type_postfix> ("&" <type_postfix>)*
<type_postfix> ::= <type_primary> (<array_suffix> | "?")*
<array_suffix> ::= "[" [<array_dimension>] "]"
<array_dimension> ::= NUMBER | "*" | IDENTIFIER
<type_primary> ::= <basic_type>
                 | "map" "<" <type_descriptor> ">"
                 | <named_type>
                 | <generic_type>
                 | <tuple_type>
                 | <record_type>
                 | <object_type>
                 | <function_type>
                 | <singleton_type>
                 | "distinct" <type_primary>
<basic_type> ::= "int" | "string" | "boolean" | "float" | "decimal" | "byte" | "anydata"
<named_type> ::= IDENTIFIER [":" IDENTIFIER]        (* optionally module-qualified *)
<generic_type> ::= <named_type> "<" <type_descriptor> ("," <type_descriptor>)* ">"
                   [ "key" "(" ... ")" ]            (* trailing table key specifier, accepted *)
<tuple_type> ::= "[" [<type_descriptor> ("," <type_descriptor>)* ["," <type_descriptor> "..."]] "]"
<record_type> ::= "record" "{" <record_member>* "}"
                | "record" "{|" <record_member>* "|}"
<record_member> ::= "*" <type_descriptor> ";"                     (* type inclusion *)
                  | <type_descriptor> "..." ";"                   (* rest field *)
                  | <type_descriptor> IDENTIFIER ["?"] ["=" <expression>] ";"
<object_type> ::= "object" "{" ... "}"             (* body currently parsed leniently *)
<function_type> ::= "function" ["(" [<type_descriptor> [IDENTIFIER]
                    ("," <type_descriptor> [IDENTIFIER])*] ")"] ["returns" <type_descriptor>]
<singleton_type> ::= ["-"] NUMBER | STRING | "true" | "false"

<function_declaration> ::= "function" <identifier> "(" <parameters> ")" ["returns" <type_descriptor>] <block>
                         | "public" "function" <identifier> "(" [<parameters>] ")" ["returns" <type_descriptor>] <block>
<parameters> ::= <parameter> ("," <parameter>)* | ε
<parameter> ::= <type_descriptor> <identifier> ["=" <expression>]

<block> ::= "{" <statement>* "}"

<statement> ::= <var_declaration>
              | <if_statement>
              | <return_statement>
              | <panic_statement>
              | <foreach_statement>
              | <while_statement>
              | <break_statement>
              | <continue_statement>
              | <match_statement>
              | <lock_statement>
              | <do_statement>
              | <transaction_statement>
              | <retry_statement>
              | <fork_statement>
              | <worker_declaration>
              | <fail_statement>
              | <rollback_statement>
              | <expression_statement>
              | <block>

<match_statement> ::= "match" <expression> "{" <match_arm>* "}"
<match_arm> ::= <match_pattern> ("|" <match_pattern>)* ["if" <expression>] "=>" <block>
<match_pattern> ::= "_" | <literal> | "var" IDENTIFIER | IDENTIFIER
                  | "[" [<match_pattern> ("," <match_pattern>)*] "]"
                  | "{" [IDENTIFIER ":" <match_pattern> ("," ...)*] "}"
                  | "error" "(" [<match_pattern> ("," <match_pattern>)*] ")"
                  | "..." IDENTIFIER
<lock_statement> ::= "lock" <block>
<do_statement> ::= "do" <block> ["on" "fail" [<type_descriptor>] IDENTIFIER <block>]
<transaction_statement> ::= "transaction" <block>
<retry_statement> ::= "retry" ["<" ... ">"] ["(" [<expression>] ")"] ["transaction"] <block>
<fork_statement> ::= "fork" <block>            (* body parsed leniently *)
<worker_declaration> ::= "worker" IDENTIFIER ["returns" <type_descriptor>] <block>
<fail_statement> ::= "fail" <expression> ";"
<rollback_statement> ::= "rollback" [<expression>] ";"

<if_statement> ::= "if" "(" <if_condition> ")" <block> ("else" <if_statement> | "else" <block>)?
<if_condition> ::= <expression>
                 | <type_descriptor> <identifier> "=" <expression>

<return_statement> ::= "return" [<expression>] ";"

<panic_statement> ::= "panic" <expression> ";"

<foreach_statement> ::= "foreach" [<type_descriptor>] <identifier> "in" <expression> <block>

<while_statement> ::= "while" <expression> <block>

<break_statement> ::= "break" ";"

<continue_statement> ::= "continue" ";"

<expression_statement> ::= <expression> ";"

<expression> ::= <assignment>

<assignment> ::= <lvalue> <assignment_op> <assignment>
               | <ternary>
<lvalue> ::= <identifier>              (* simple variable target *)
           | <postfix>                 (* field/index target, e.g. self.x or a[i] *)
<assignment_op> ::= "=" | "+=" | "-="

<ternary> ::= <logic_or> ("?" <logic_or> ":" <ternary>)?
            | <logic_or> "?:" <logic_or>

<logic_or> ::= <logic_and> ("||" <logic_and>)*

<logic_and> ::= <equality> ("&&" <equality>)*

<equality> ::= <comparison> (("==" | "!=" | "===" | "!==") <comparison>)*

<comparison> ::= <shift> ((">" | ">=" | "<" | "<=" | "is") <shift>)*

<bitwise_or> ::= <bitwise_xor> ("|" <bitwise_xor>)*

<bitwise_xor> ::= <bitwise_and> ("^" <bitwise_and>)*

<bitwise_and> ::= <shift> ("&" <shift>)*

<shift> ::= <additive> (("<<" | ">>" | ">>>") <additive>)*

<additive> ::= <multiplicative> (("+" | "-") <multiplicative>)*

<multiplicative> ::= <unary> (("*" | "/" | "%") <unary>)*

<unary> ::= ("!" | "-" | "~" | "+") <unary>
          | ("check" | "checkpanic" | "trap") <unary>
          | "typeof" <unary>
          | <let_expression>
          | <postfix>
<let_expression> ::= "let" <let_binding> ("," <let_binding>)* "in" <expression>
<let_binding> ::= ["final"] <type_descriptor> IDENTIFIER "=" <expression>

<postfix> ::= <primary> <postfix_op>*
<postfix_op> ::= "[" <expression> "]"
               | "." <identifier> "(" [<call_arguments>] ")"   (* method call *)
               | "." <identifier>                              (* field access *)
               | "->" <identifier> "(" [<call_arguments>] ")"  (* remote call *)
               | ":" <identifier> "(" [<call_arguments>] ")"
               | "(" [<call_arguments>] ")"

<call_arguments> ::= <positional_arguments>
                   | <named_arguments>
<positional_arguments> ::= <expression> ("," <expression>)*
<named_arguments> ::= <identifier> "=" <expression> ("," <identifier> "=" <expression>)*

<primary> ::= <number_literal>
            | <string_literal>
            | <string_template>
            | "true"
            | "false"
            | "()"
            | <identifier>
            | <array_literal>
            | <map_literal>
            | "(" <expression> ")"
            | <range_expression>
            | <cast_expression>
            | <new_expression>
            | <anonymous_function>
            | <arrow_function>
            | <query_expression>
            | <table_constructor>
            | <start_action>
<query_expression> ::= <from_clause> <query_clause>* <select_clause> [<on_conflict_clause>]
<from_clause> ::= "from" <typed_binding_pattern> "in" <expression>
<query_clause> ::= <from_clause>
                 | "where" <expression>
                 | "let" <let_binding> ("," <let_binding>)*
                 | ["outer"] "join" <typed_binding_pattern> "in" <expression>
                   "on" <expression> "equals" <expression>
                 | "order" "by" <order_key> ("," <order_key>)*
                 | "limit" <expression>
                 | "group" "by" <expression>
<order_key> ::= <expression> ["ascending" | "descending"]
<select_clause> ::= "select" <expression>
<on_conflict_clause> ::= "on" "conflict" <expression>
<table_constructor> ::= "table" ["key" "(" ... ")"] "[" [<expression> ("," <expression>)*] "]"
<start_action> ::= "start" <postfix>
<new_expression> ::= "new" [<type_descriptor>] ["(" [<call_arguments>] ")"]
<anonymous_function> ::= "function" "(" [<parameters>] ")" ["returns" <type_descriptor>] <block>
<arrow_function> ::= <identifier> "=>" <expression>
                   | "(" [<arrow_param> ("," <arrow_param>)*] ")" "=>" <expression>
<arrow_param> ::= [<type_descriptor>] IDENTIFIER

<number_literal> ::= NUMBER [<numeric_suffix>]
<numeric_suffix> ::= "f" | "F" | "d" | "D"

<string_literal> ::= DOUBLE_QUOTE_STRING
<string_template> ::= "`" <template_part>* "`"
<template_part> ::= STRING_CHAR
                  | "${" <expression> "}"

<array_literal> ::= "[" [<expression> ("," <expression>)*] "]"

<map_literal> ::= "{" [<map_entry> ("," <map_entry>)*] "}"
<map_entry> ::= STRING ":" <expression>

<range_expression> ::= <expression> "..." <expression>

<cast_expression> ::= "<" <type_descriptor> ">" <expression>

<identifier> ::= IDENTIFIER
               | "'" IDENTIFIER
               | IDENTIFIER ("\\" CHAR)*
```