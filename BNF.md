```bnf
<program> ::= <statement>*

<statement> ::= <var_declaration>
              | <function_declaration>
              | <if_statement>
              | <return_statement>
              | <panic_statement>
              | <expression_statement>

<var_declaration> ::= "var" <identifier> ":" <type> "=" <expression> ";"

<function_declaration> ::= "function" <identifier> "(" <parameters> ")" "returns" <type> <block>

<parameters> ::= <parameter> ("," <parameter>)* | ε

<parameter> ::= <identifier> ":" <type>

<type> ::= "int" | "string" | "boolean" | "float"

<if_statement> ::= "if" "(" <expression> ")" <block> ("else" <block>)?

<while_statement> ::= "while" "(" <expression> ")" <block>

<foreach_statement> ::= "foreach" "(" <identifier> "in" <expression> ")" <block>

<return_statement> ::= "return" <expression>? ";"

<panic_statement> ::= "panic" <expression> ";"

<check_statement> ::= "check" <expression> ";"

<expression_statement> ::= <expression> ";"

<block> ::= "{" <statement>* "}"

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