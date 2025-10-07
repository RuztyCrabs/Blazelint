```bnf
<program> ::= <declaration>*

<declaration> ::= <var_declaration>
                | <function_declaration>
                | <statement>

<var_declaration> ::= <local_init_var_decl>
                    | <local_no_init_var_decl>

<local_init_var_decl> ::= ["final"] <typed_binding_pattern> "=" <expression> ";"
<local_no_init_var_decl> ::= ["final"] <type_descriptor> <identifier> ";"

<typed_binding_pattern> ::= "var" <identifier>
                          | <type_descriptor> <identifier>

<type_descriptor> ::= "int" | "string" | "boolean" | "float" | <identifier>

<function_declaration> ::= "function" <identifier> "(" <parameters> ")" ["returns" <type_descriptor>] <block>

<parameters> ::= <parameter> ("," <parameter>)* | ε

<parameter> ::= <identifier> ":" <type_descriptor>

<statement> ::= <if_statement>
              | <return_statement>
              | <panic_statement>
              | <expression_statement>

<if_statement> ::= "if" "(" <expression> ")" <block> ("else" <block>)?

<return_statement> ::= "return" <expression>? ";"

<panic_statement> ::= "panic" <expression> ";"

<expression_statement> ::= <expression> ";"

<block> ::= "{" <declaration>* "}"

<expression> ::= <assignment>

<assignment> ::= <identifier> "=" <assignment> | <logic_or>

<logic_or> ::= <logic_and> ("||" <logic_and>)*

<logic_and> ::= <equality> ("&&" <equality>)*

<equality> ::= <comparison> (("!=" | "==") <comparison>)*

<comparison> ::= <term> ((">" | ">=" | "<" | "<=") <term>)*

<term> ::= <factor> (("+" | "-") <factor>)*

<factor> ::= <unary> (("*" | "/") <unary>)*

<unary> ::= ("!" | "-") <unary> | <primary>

<primary> ::= <number_literal>
            | <string_literal>
            | "true"
            | "false"
            | <identifier>
            | "(" <expression> ")"

<number_literal> ::= NUMBER
<string_literal> ::= STRING
<identifier> ::= IDENTIFIER
```
