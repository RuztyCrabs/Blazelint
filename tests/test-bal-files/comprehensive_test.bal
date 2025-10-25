// Comprehensive test file for Blazelint parsable syntax
// Tests all currently supported language features

import ballerina/io;

// Constants with proper naming
const MAX_SIZE = 100;
const DEFAULT_NAME = "test";
const PI_VALUE = 3.14;

// Public function with parameters and return type
public function main() {
    variable_declarations();
    operator_tests();
    control_flow_tests();
    other_tests();
    
    // Return statement (in main, returns nil)
    return;
}

function variable_declarations() {
    // Variable declarations with type annotations
    int x = 5;
    float y = 3.14;
    boolean flag = true;
    string message = "Hello, World!";
    
    // Final variables
    final int constant = 42;
    final string name = "Ballerina";
    
    // Array declarations
    int[] numbers = [1, 2, 3, 4, 5];
    string[] names = ["Alice", "Bob", "Charlie"];
    
    // Map declarations
    map<string> config = {name: "app", version: "1.0"};
    map<int> scores = {math: 90, science: 85};
}

function operator_tests() {
    int x = 5;
    float y = 3.14;
    boolean flag = true;
    int sum = 30;
    int diff = 20;

    // Arithmetic operators
    int sum_op = 10 + 20;
    int diff_op = 50 - 30;
    int product = 5 * 4;
    int remainder = 10 % 3;
    
    // Comparison operators
    boolean isEqual = x == 5;
    boolean notEqual = y != 0.0;
    boolean greater = sum > diff;
    boolean greaterEqual = x >= 5;
    boolean lessEqual = y <= 10.0;
    
    // Logical operators
    boolean andResult = flag && isEqual;
    boolean orResult = (x > 0) || (y < 0.0);
    boolean notResult = !flag;
    
    // Bitwise operators
    int bitwiseAnd = 5 & 3;
    int bitwiseOr = 5 | 3;
    int bitwiseXor = 5 ^ 3;
    int bitwiseNot = ~5;
    
    // Shift operators
    int leftShift = 4 << 2;
    int rightShift = 16 >> 2;
    int unsignedRightShift = -16 >>> 2;
    
    // Compound assignment
    int counter = 0;
    counter += 5;
    counter -= 2;
    
    // Ternary operator
    int max = (x > 5) ? x : 10;
}

function control_flow_tests() {
    int x = 5;
    boolean flag = true;
    int[] numbers = [1, 2, 3, 4, 5];
    string[] names = ["Alice", "Bob", "Charlie"];

    // If statement
    if (x > 0) {
        io:println("Positive");
    }
    
    // If-else statement
    if (flag) {
        io:println("True branch");
    } else {
        io:println("False branch");
    }
    
    // Nested if-else
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
    
    // Foreach loop with type annotation
    foreach int num in numbers {
        io:println(num);
    }
    
    // Foreach with string array
    foreach string personName in names {
        io:println(personName);
    }
    
    // Break and continue in loops
    int j = 0;
    while (j < 10) {
        if (j == 3) {
            j += 1;
            continue;
        }
        if (j == 7) {
            break;
        }
        io:println(j);
        j += 1;
    }
}

function other_tests() {
    int x = 5;
    string message = "Hello, World!";
    int[] numbers = [1, 2, 3, 4, 5];
    string[] names = ["Alice", "Bob", "Charlie"];
    map<string> config = {name: "app", version: "1.0"};

    // Array access
    int firstNumber = numbers[0];
    string firstName = names[1];
    
    // Map access
    string? appName = config["name"];
    
    // Method calls
    names.push("David");
    int length = names.length();
    
    // Function calls
    io:println(message);
    io:println(numbers);
    io:println(names);
    
    calculate(10, 20);
    
    // Type casts
    int intValue = 42;
    float floatValue = <float>intValue;
    
    // String templates (basic)
    string greeting = "Hello";
    
    // Grouped expressions
    int calculation = (10 + 20) * (5 - 2);
    boolean complexCondition = ((x > 0) && (x < 100)) || (x > 200);
    
    // Unary operators
    int negative = -x;
    int positive = +10;
    boolean inverted = !false;
    
    // Multiple assignments
    int a = 1;
    int b = 2;
    int c = 3;
}

// Function with parameters and return type
function add(int a, int b) returns int {
    return a + b;
}

// Function without return type (returns nil)
function calculate(int x, int y) {
    int result = x + y;
    io:println(result);
}

// Function with final parameters
function multiply(int a, int b) returns int {
    final int constA = a;
    final int constB = b;
    return constA * constB;
}

// Function with boolean return
function isPositive(int n) returns boolean {
    if (n > 0) {
        return true;
    }
    return false;
}

// Function with string return
function getGreeting(string name) returns string {
    return "Hello";
}

// Function with array parameter
function sumArray(int[] arr) returns int {
    int total = 0;
    int index = 0;
    while (index < 10) {
        index += 1;
    }
    return total;
}

// Function with multiple returns
function minMax(int a, int b) returns int {
    if (a < b) {
        return a;
    } else {
        return b;
    }
}

// Public function
public function publicHelper() returns string {
    return "public";
}

// Function demonstrating all expression types
function expressionDemo() {
    // Literals
    int intLit = 42;
    float floatLit = 3.14;
    boolean boolLit = true;
    string strLit = "text";
    
    // Arrays and maps
    int[] arr = [1, 2, 3];
    map<string> m = {key: "value"};
    
    // Binary operations
    boolean comp = (5 > 3) && (2 < 4);
    int bitwise = (8 & 4) | (2 ^ 1);
    int shift = (1 << 3) >> 1;
    
    // Unary operations
    int neg = -5;
    boolean not = !true;
    int bitnot = ~15;
    
    // Ternary
    int tern = (intLit > 0) ? 1 : 0;
    
    // Grouped
    int grouped = (1 + 2) * 3;
    
    io:println("Expression demo complete");
}