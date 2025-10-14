<program> ::= <import_declaration>* <module_level_declaration>*

<import_declaration> ::= "import" <package_name> ";"
<package_name> ::= IDENTIFIER ("/" IDENTIFIER)*

<module_level_declaration> ::= <var_declaration>
                             | <function_declaration>

<var_declaration> ::= ["final"] <typed_binding_pattern> "=" <expression> ";"
                    | ["final"] <type_descriptor> <identifier> ";"
<typed_binding_pattern> ::= "var" <identifier>
                          | <type_descriptor> <identifier>

<type_descriptor> ::= <basic_type> <type_suffix>*
                    | "map" "<" <type_descriptor> ">"
                    | IDENTIFIER <type_suffix>*
<basic_type> ::= "int" | "string" | "boolean" | "float" | "decimal" | "byte" | "anydata"
<type_suffix> ::= "[" [<array_dimension>] "]"
                | "?"
                | "|" <type_descriptor>
<array_dimension> ::= NUMBER | "*"

<function_declaration> ::= "function" <identifier> "(" <parameters> ")" ["returns" <type_descriptor>] <block>
                         | "public" "function" "main" "(" [<parameters>] ")" ["returns" <type_descriptor>] <block>
<parameters> ::= <parameter> ("," <parameter>)* | ε
<parameter> ::= <identifier> ":" <type_descriptor> ["=" <expression>]

<block> ::= "{" <statement>* "}"

<statement> ::= <var_declaration>
              | <if_statement>
              | <return_statement>
              | <foreach_statement>
              | <while_statement>
              | <break_statement>
              | <continue_statement>
              | <expression_statement>
              | <block>

<if_statement> ::= "if" "(" <if_condition> ")" <block> ("else" <if_statement> | "else" <block>)?
<if_condition> ::= <expression>
                 | <type_descriptor> <identifier> "=" <expression>

<return_statement> ::= "return" [<expression>] ";"

<foreach_statement> ::= "foreach" <type_descriptor> <identifier> "in" <expression> <block>

<while_statement> ::= "while" <expression> <block>

<break_statement> ::= "break" ";"

<continue_statement> ::= "continue" ";"

<expression_statement> ::= <expression> ";"

<expression> ::= <assignment>

<assignment> ::= <identifier> <assignment_op> <assignment>
               | <ternary>
<assignment_op> ::= "=" | "+=" | "-="

<ternary> ::= <logic_or> ("?" <logic_or> ":" <ternary>)?
            | <logic_or> "?:" <logic_or>

<logic_or> ::= <logic_and> ("||" <logic_and>)*

<logic_and> ::= <equality> ("&&" <equality>)*

<equality> ::= <comparison> (("==" | "!=" | "===" | "!==") <comparison>)*

<comparison> ::= <bitwise_or> ((">" | ">=" | "<" | "<=" | "is") <bitwise_or>)*

<bitwise_or> ::= <bitwise_xor> ("|" <bitwise_xor>)*

<bitwise_xor> ::= <bitwise_and> ("^" <bitwise_and>)*

<bitwise_and> ::= <shift> ("&" <shift>)*

<shift> ::= <additive> (("<<" | ">>" | ">>>") <additive>)*

<additive> ::= <multiplicative> (("+" | "-") <multiplicative>)*

<multiplicative> ::= <unary> (("*" | "/") <unary>)*

<unary> ::= ("!" | "-" | "~" | "+") <unary>
          | <postfix>

<postfix> ::= <primary> <postfix_op>*
<postfix_op> ::= "[" <expression> "]"
               | "." <identifier> "(" [<call_arguments>] ")"
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