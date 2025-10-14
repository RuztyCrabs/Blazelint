// Test file to verify error recovery - should report all errors at once

import ballerina/io;

// Error 1: Missing semicolon
int x = 5

// Error 2: Invalid function name (not camelCase)
function BAD_NAME() {
    io:println("test");
}

// Error 3: Missing closing brace - this will be severe
function goodFunction() {
    int y = 10;
    if (y > 5) {
        io:println("yes");
    // Missing closing brace for if
    
// Error 4: Missing parameter type
function brokenParams(x, int y) returns int {
    return x + y;
}

// Error 5: Constant not in CONSTANT_CASE
const badConstant = 100;

// This should still parse fine
function workingFunction() returns int {
    return 42;
}
