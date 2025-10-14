// Simple test showing multiple errors caught in one pass

import ballerina/io;

// Error 1: Missing semicolon after first declaration
int x = 5

// Error 2: Another missing semicolon
int y = 10

// Error 3: Bad constant naming (should be SCREAMING_SNAKE_CASE)
const myConstant = 42;

// Error 4: Bad function naming (should be camelCase)  
function BAD_FUNCTION_NAME() {
    io:println("test");
}

// This function is correct and should parse fine
function goodFunction() returns int {
    return 100;
}
