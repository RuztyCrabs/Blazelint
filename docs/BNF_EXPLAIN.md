# BNF Grammar Explanation with Examples

This document explains each part of the Blazelint BNF grammar with corresponding Ballerina code examples.

---

## Program Structure

### `<program>`
```bnf
<program> ::= <import_declaration>* <module_level_declaration>*
```

A Ballerina program consists of zero or more import declarations followed by zero or more module-level declarations.

**Example:**
```ballerina
import ballerina/io;
import ballerina/http;

const MAX_SIZE = 100;

function main() {
    io:println("Hello, World!");
}
```

---

## Import Declarations

### `<import_declaration>`
```bnf
<import_declaration> ::= "import" <package_name> ";"
<package_name> ::= IDENTIFIER ("/" IDENTIFIER)*
```

Import statements bring external packages into scope. Package names can have multiple path segments separated by `/`.

**Examples:**
```ballerina
import ballerina/io;
import ballerina/http;
import ballerina/lang.array;
import myorg/mypackage.utils;
```

---

## Module-Level Declarations

### `<module_level_declaration>`
```bnf
<module_level_declaration> ::= <var_declaration>
                             | <const_declaration>
                             | <function_declaration>
```

Top-level declarations can be variables, constants, or functions.

**Examples:**
```ballerina
// Variable declaration
int globalCounter = 0;

// Constant declaration
const API_KEY = "secret-key-123";

// Function declaration
function helper() {
    // function body
}
```

---

## Variable Declarations

### `<var_declaration>`
```bnf
<var_declaration> ::= ["final"] <typed_binding_pattern> "=" <expression> ";"
                    | ["final"] <type_descriptor> <identifier> ";"
```

Variables can be declared with or without initialization, and can optionally be marked as `final`.

**Examples:**
```ballerina
// Type inference with var
var x = 42;
var message = "Hello";

// Explicit type with initialization
int count = 10;
string name = "Alice";

// Explicit type without initialization
int total;
boolean flag;

// Final variables (immutable after initialization)
final int MAX = 100;
final var PI = 3.14;
```

### `<typed_binding_pattern>`
```bnf
<typed_binding_pattern> ::= "var" <identifier>
                          | <type_descriptor> <identifier>
```

Variables can use type inference (`var`) or explicit type annotations.

**Examples:**
```ballerina
var x = 5;              // Type inferred as int
int y = 10;             // Explicit int type
string[] names = [];    // Explicit array type
```

---

## Constant Declarations

### `<const_declaration>`
```bnf
<const_declaration> ::= "const" <identifier> "=" <expression> ";"
```

Constants are immutable values that must be initialized at declaration.

**Examples:**
```ballerina
const MAX_SIZE = 1000;
const DEFAULT_NAME = "Ballerina";
const PI_VALUE = 3.14159;
const TIMEOUT = 30;
```

---

## Type Descriptors

### `<type_descriptor>`
```bnf
<type_descriptor> ::= <basic_type> <type_suffix>*
                    | "map" "<" <type_descriptor> ">"
                    | IDENTIFIER <type_suffix>*
```

Type descriptors define the types of variables, parameters, and return values.

### `<basic_type>`
```bnf
<basic_type> ::= "int" | "string" | "boolean" | "float" | "decimal" | "byte" | "anydata"
```

**Examples:**
```ballerina
int age = 25;
string name = "Bob";
boolean isActive = true;
float price = 19.99;
decimal precise = 123.456;
byte statusCode = 200;
anydata flexible = "anything";
```

### `<type_suffix>`
```bnf
<type_suffix> ::= "[" [<array_dimension>] "]"
                | "?"
                | "|" <type_descriptor>
```

Type suffixes add array dimensions, optional types, or union types.

**Examples:**
```ballerina
// Arrays
int[] numbers = [1, 2, 3];
string[3] names = ["A", "B", "C"];
int[*] inferred = [1, 2, 3, 4, 5];

// Optional types (not fully implemented)
// int? maybeNull = ();

// Union types
// int|string flexible = 42;
```

### `<array_dimension>`
```bnf
<array_dimension> ::= NUMBER | "*"
```

Arrays can have fixed size, inferred size (`*`), or open size.

**Examples:**
```ballerina
int[5] fixedSize = [1, 2, 3, 4, 5];
int[*] inferred = [1, 2, 3];
int[] openSize = [1, 2, 3, 4];
```

### Map Types
**Examples:**
```ballerina
map<string> config = {name: "app", version: "1.0"};
map<int> scores = {math: 90, science: 85};
map<float> prices = {apple: 1.99, banana: 0.99};
```

---

## Function Declarations

### `<function_declaration>`
```bnf
<function_declaration> ::= "function" <identifier> "(" <parameters> ")" ["returns" <type_descriptor>] <block>
                         | "public" "function" <identifier> "(" [<parameters>] ")" ["returns" <type_descriptor>] <block>
```

Functions can be private (default) or public, with optional parameters and return types.

**Examples:**
```ballerina
// Simple function
function greet() {
    io:println("Hello!");
}

// Function with parameters
function add(int a, int b) returns int {
    return a + b;
}

// Public function
public function main() {
    io:println("Starting...");
}

// Function with no return type
function calculate(int x, int y) {
    int result = x + y;
    io:println(result);
}
```

### `<parameters>`
```bnf
<parameters> ::= <parameter> ("," <parameter>)* | ε
<parameter> ::= <type_descriptor> <identifier> ["=" <expression>]
```

Parameters specify the inputs to functions. Note: parameter syntax is `type name`, not `name: type`.

**Examples:**
```ballerina
// No parameters
function doSomething() { }

// Single parameter
function square(int x) returns int {
    return x * x;
}

// Multiple parameters
function multiply(int a, int b) returns int {
    return a * b;
}

// Parameters with default values (not fully implemented)
// function greet(string name = "World") { }
```

---

## Statements

### `<statement>`
```bnf
<statement> ::= <var_declaration>
              | <if_statement>
              | <return_statement>
              | <panic_statement>
              | <foreach_statement>
              | <while_statement>
              | <break_statement>
              | <continue_statement>
              | <expression_statement>
              | <block>
```

### `<block>`
```bnf
<block> ::= "{" <statement>* "}"
```

Blocks group multiple statements together.

**Example:**
```ballerina
function example() {
    {
        int x = 5;
        int y = 10;
        io:println(x + y);
    }
}
```

---

## Control Flow Statements

### `<if_statement>`
```bnf
<if_statement> ::= "if" "(" <if_condition> ")" <block> ("else" <if_statement> | "else" <block>)?
<if_condition> ::= <expression>
                 | <type_descriptor> <identifier> "=" <expression>
```

If statements allow conditional execution with optional else-if chains and else blocks.

**Examples:**
```ballerina
// Simple if
if (x > 0) {
    io:println("Positive");
}

// If-else
if (x > 0) {
    io:println("Positive");
} else {
    io:println("Non-positive");
}

// If-else-if-else chain
if (x > 10) {
    io:println("Greater than 10");
} else if (x > 5) {
    io:println("Between 5 and 10");
} else {
    io:println("5 or less");
}

// Type test (not fully implemented)
// if (int result = getValue()) {
//     io:println(result);
// }
```

### `<while_statement>`
```bnf
<while_statement> ::= "while" <expression> <block>
```

While loops execute repeatedly while a condition is true.

**Examples:**
```ballerina
int i = 0;
while (i < 5) {
    io:println(i);
    i += 1;
}

// Infinite loop with break
while (true) {
    if (condition) {
        break;
    }
}
```

### `<foreach_statement>`
```bnf
<foreach_statement> ::= "foreach" [<type_descriptor>] <identifier> "in" <expression> <block>
```

Foreach loops iterate over collections. Type annotation is optional.

**Examples:**
```ballerina
int[] numbers = [1, 2, 3, 4, 5];

// With type annotation
foreach int num in numbers {
    io:println(num);
}

// Without type annotation (type inferred)
foreach n in numbers {
    io:println(n);
}

// Iterating over strings
string[] names = ["Alice", "Bob", "Charlie"];
foreach string name in names {
    io:println(name);
}
```

### `<break_statement>` and `<continue_statement>`
```bnf
<break_statement> ::= "break" ";"
<continue_statement> ::= "continue" ";"
```

**Examples:**
```ballerina
int i = 0;
while (i < 10) {
    if (i == 3) {
        i += 1;
        continue;  // Skip to next iteration
    }
    if (i == 7) {
        break;     // Exit loop
    }
    io:println(i);
    i += 1;
}
```

### `<return_statement>`
```bnf
<return_statement> ::= "return" [<expression>] ";"
```

Return statements exit functions and optionally provide a return value.

**Examples:**
```ballerina
// Return with value
function add(int a, int b) returns int {
    return a + b;
}

// Return without value
function doSomething() {
    io:println("Done");
    return;
}

// Multiple returns
function max(int a, int b) returns int {
    if (a > b) {
        return a;
    } else {
        return b;
    }
}
```

### `<panic_statement>`
```bnf
<panic_statement> ::= "panic" <expression> ";"
```

Panic statements cause the program to terminate with an error.

**Examples:**
```ballerina
function divide(int a, int b) returns int {
    if (b == 0) {
        panic error("Division by zero!");
    }
    return a / b;
}
```

### `<expression_statement>`
```bnf
<expression_statement> ::= <expression> ";"
```

Any expression can be used as a statement.

**Examples:**
```ballerina
x + 5;                    // Expression statement
io:println("Hello");      // Function call statement
counter += 1;             // Assignment statement
```

---

## Expressions

### Expression Hierarchy
```bnf
<expression> ::= <assignment>
<assignment> ::= <identifier> <assignment_op> <assignment>
               | <ternary>
<assignment_op> ::= "=" | "+=" | "-="
```

**Examples:**
```ballerina
// Simple assignment
x = 10;

// Compound assignment
counter += 5;
total -= 2;

// Chained assignment
x = y = z = 0;
```

### `<ternary>`
```bnf
<ternary> ::= <logic_or> ("?" <logic_or> ":" <ternary>)?
            | <logic_or> "?:" <logic_or>
```

Ternary conditional operator and Elvis operator.

**Examples:**
```ballerina
// Ternary operator
int max = (a > b) ? a : b;
string status = (age >= 18) ? "adult" : "minor";

// Elvis operator (not fully implemented)
// int value = maybeNull ?: 42;
```

### `<logic_or>` and `<logic_and>`
```bnf
<logic_or> ::= <logic_and> ("||" <logic_and>)*
<logic_and> ::= <equality> ("&&" <equality>)*
```

Logical OR has lower precedence than logical AND.

**Examples:**
```ballerina
boolean result1 = (x > 0) && (y < 10);
boolean result2 = (a == 0) || (b == 0);
boolean result3 = (x > 5) && (y > 5) || (z > 5);
```

### `<equality>`
```bnf
<equality> ::= <comparison> (("==" | "!=" | "===" | "!==") <comparison>)*
```

Equality and inequality comparisons.

**Examples:**
```ballerina
boolean isEqual = (x == 5);
boolean notEqual = (y != 0);
boolean exactlyEqual = (a === b);      // Reference equality
boolean notExactlyEqual = (a !== b);   // Reference inequality
```

### `<comparison>`
```bnf
<comparison> ::= <shift> ((">" | ">=" | "<" | "<=" | "is") <shift>)*
```

Relational comparisons and type checking.

**Examples:**
```ballerina
boolean greater = (x > 5);
boolean greaterEqual = (y >= 10);
boolean less = (a < b);
boolean lessEqual = (c <= d);

// Type checking (not fully implemented)
// boolean isInt = (value is int);
```

### Bitwise Operations
```bnf
<bitwise_or> ::= <bitwise_xor> ("|" <bitwise_xor>)*
<bitwise_xor> ::= <bitwise_and> ("^" <bitwise_and>)*
<bitwise_and> ::= <shift> ("&" <shift>)*
```

**Examples:**
```ballerina
int bitwiseAnd = 5 & 3;      // 0101 & 0011 = 0001 (1)
int bitwiseOr = 5 | 3;       // 0101 | 0011 = 0111 (7)
int bitwiseXor = 5 ^ 3;      // 0101 ^ 0011 = 0110 (6)
```

### `<shift>`
```bnf
<shift> ::= <additive> (("<<" | ">>" | ">>>") <additive>)*
```

Bit shift operations.

**Examples:**
```ballerina
int leftShift = 4 << 2;           // 4 * 2^2 = 16
int rightShift = 16 >> 2;         // 16 / 2^2 = 4
int unsignedRight = -16 >>> 2;    // Unsigned right shift
```

### `<additive>` and `<multiplicative>`
```bnf
<additive> ::= <multiplicative> (("+" | "-") <multiplicative>)*
<multiplicative> ::= <unary> (("*" | "/" | "%") <unary>)*
```

Arithmetic operations with proper precedence.

**Examples:**
```ballerina
int sum = 10 + 20;
int difference = 50 - 30;
int product = 5 * 4;
float quotient = 100.0 / 5.0;
int remainder = 10 % 3;

// Precedence: multiplication before addition
int result = 2 + 3 * 4;  // 2 + 12 = 14
```

### `<unary>`
```bnf
<unary> ::= ("!" | "-" | "~" | "+") <unary>
          | <postfix>
```

Unary operators for negation, bitwise NOT, and explicit positive.

**Examples:**
```ballerina
boolean notFlag = !true;           // Logical NOT
int negative = -42;                // Numeric negation
int positive = +10;                // Explicit positive
int bitwiseNot = ~5;               // Bitwise NOT (inverts bits)
```

---

## Postfix Operations

### `<postfix>`
```bnf
<postfix> ::= <primary> <postfix_op>*
<postfix_op> ::= "[" <expression> "]"
               | "." <identifier> "(" [<call_arguments>] ")"
               | ":" <identifier> "(" [<call_arguments>] ")"
               | "(" [<call_arguments>] ")"
```

Postfix operations include array/map access, method calls, qualified calls, and function calls.

**Examples:**
```ballerina
// Array access
int firstNumber = numbers[0];
string lastName = names[2];

// Method calls
int length = names.length();
names.push("David");
int popped = numbers.pop();

// Function calls
io:println("Hello");
int sum = add(5, 3);

// Qualified calls (module:function)
io:println("Message");
http:get("https://example.com");
```

### `<call_arguments>`
```bnf
<call_arguments> ::= <positional_arguments>
                   | <named_arguments>
<positional_arguments> ::= <expression> ("," <expression>)*
<named_arguments> ::= <identifier> "=" <expression> ("," <identifier> "=" <expression>)*
```

Function arguments can be positional or named.

**Examples:**
```ballerina
// Positional arguments
io:println("Hello", "World");
int sum = add(5, 10);

// Named arguments (not fully implemented)
// greet(name="Alice", age=25);
```

---

## Primary Expressions

### `<primary>`
```bnf
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
```

Primary expressions are the basic building blocks of expressions.

### Literals

#### `<number_literal>`
```bnf
<number_literal> ::= NUMBER [<numeric_suffix>]
<numeric_suffix> ::= "f" | "F" | "d" | "D"
```

**Examples:**
```ballerina
int integer = 42;
float floatingPoint = 3.14;
float scientific = 1.5e-3;
float withSuffix = 3.14f;
decimal precise = 123.456d;
```

#### `<string_literal>` and `<string_template>`
```bnf
<string_literal> ::= DOUBLE_QUOTE_STRING
<string_template> ::= "`" <template_part>* "`"
<template_part> ::= STRING_CHAR
                  | "${" <expression> "}"
```

**Examples:**
```ballerina
string simple = "Hello, World!";
string escaped = "She said, \"Hello\"";

// String templates (basic support)
string template = `Hello, World!`;
// string interpolated = `Value: ${x}`;  // Interpolation not fully implemented
```

#### Boolean and Nil Literals

**Examples:**
```ballerina
boolean trueValue = true;
boolean falseValue = false;
var nothing = ();  // Nil literal
```

#### `<identifier>`
**Examples:**
```ballerina
int myVariable = 10;
string userName = "Alice";
var _privateVar = 5;
int camelCaseExample = 100;
```

### `<array_literal>`
```bnf
<array_literal> ::= "[" [<expression> ("," <expression>)*] "]"
```

**Examples:**
```ballerina
int[] numbers = [1, 2, 3, 4, 5];
string[] names = ["Alice", "Bob", "Charlie"];
float[] prices = [1.99, 2.99, 3.99];
int[] empty = [];

// Nested arrays
int[][] matrix = [[1, 2], [3, 4], [5, 6]];
```

### `<map_literal>`
```bnf
<map_literal> ::= "{" [<map_entry> ("," <map_entry>)*] "}"
<map_entry> ::= STRING ":" <expression>
```

**Examples:**
```ballerina
map<string> config = {
    name: "MyApp",
    version: "1.0.0"
};

map<int> scores = {
    math: 90,
    science: 85,
    english: 88
};

map<string> empty = {};
```

### Grouped Expressions

**Examples:**
```ballerina
int result = (2 + 3) * 4;        // Grouping changes precedence
boolean check = ((x > 0) && (y < 10)) || (z == 5);
```

### `<range_expression>` (Not Fully Implemented)
```bnf
<range_expression> ::= <expression> "..." <expression>
```

**Examples:**
```ballerina
// Range expressions for iteration
// int[] range = 1...10;
// foreach int i in 0...5 { }
```

### `<cast_expression>`
```bnf
<cast_expression> ::= "<" <type_descriptor> ">" <expression>
```

Type casting to convert between types.

**Examples:**
```ballerina
int intValue = 42;
float floatValue = <float>intValue;

string strValue = "123";
// int parsed = <int>strValue;  // Type casting
```

---

## Operator Precedence (Highest to Lowest)

1. **Primary**: Literals, identifiers, grouping `()`
2. **Postfix**: `[]`, `.`, function calls `()`
3. **Unary**: `!`, `-`, `+`, `~`
4. **Multiplicative**: `*`, `/`, `%`
5. **Additive**: `+`, `-`
6. **Shift**: `<<`, `>>`, `>>>`
7. **Bitwise AND**: `&`
8. **Bitwise XOR**: `^`
9. **Bitwise OR**: `|`
10. **Comparison**: `<`, `<=`, `>`, `>=`, `is`
11. **Equality**: `==`, `!=`, `===`, `!==`
12. **Logical AND**: `&&`
13. **Logical OR**: `||`
14. **Ternary/Elvis**: `? :`, `?:`
15. **Assignment**: `=`, `+=`, `-=`

**Example demonstrating precedence:**
```ballerina
int result = 2 + 3 * 4 > 10 && true || false;
// Evaluation order:
// 1. 3 * 4 = 12
// 2. 2 + 12 = 14
// 3. 14 > 10 = true
// 4. true && true = true
// 5. true || false = true
```

---

## Complete Example

Here's a comprehensive example using many features:

```ballerina
import ballerina/io;

// Constants
const MAX_USERS = 1000;
const DEFAULT_ROLE = "guest";

// Global variable
int userCount = 0;

// Public main function
public function main() {
    // Variable declarations
    int x = 5;
    string name = "Alice";
    final int TIMEOUT = 30;
    
    // Arrays and maps
    int[] scores = [85, 90, 95];
    map<string> config = {host: "localhost", port: "8080"};
    
    // Arithmetic operations
    int sum = 10 + 20;
    int product = 5 * 4;
    int remainder = 10 % 3;
    
    // Bitwise operations
    int bitwiseAnd = 5 & 3;
    int leftShift = 4 << 2;
    
    // Comparison and logical
    boolean isValid = (x > 0) && (x < 100);
    
    // Ternary operator
    string status = (x > 50) ? "high" : "low";
    
    // If-else-if-else
    if (x > 10) {
        io:println("Greater than 10");
    } else if (x > 5) {
        io:println("Between 5 and 10");
    } else {
        io:println("5 or less");
    }
    
    // While loop
    int i = 0;
    while (i < 5) {
        io:println(i);
        i += 1;
    }
    
    // Foreach loop
    foreach int score in scores {
        io:println(score);
    }
    
    // Function calls
    int result = add(5, 10);
    io:println(result);
    
    // Method calls
    scores.push(100);
    int length = scores.length();
    
    // Type casting
    float floatValue = <float>x;
}

// Function with parameters and return type
function add(int a, int b) returns int {
    return a + b;
}

// Function with multiple returns
function max(int a, int b) returns int {
    if (a > b) {
        return a;
    } else {
        return b;
    }
}
```

---

## Notes on Implementation Status

- ✅ **Fully Implemented**: Basic types, arrays, maps, operators, control flow, functions
- ⚠️ **Partially Implemented**: 
  - String templates (basic parsing, no interpolation evaluation)
  - Type casting (parsed but not fully type-checked)
  - Range expressions (parsed but not type-checked)
- ❌ **Not Implemented**: 
  - Optional types (`int?`)
  - Union types (`int|string`)
  - Named function arguments
  - Default parameter values
  - Type test in if conditions

This BNF represents the **currently parsable and type-checked subset** of Ballerina syntax supported by Blazelint.
