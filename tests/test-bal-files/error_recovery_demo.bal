// Comprehensive error recovery test
// All these errors should be reported in ONE RUN!

import ballerina/io;

// Parser Error 1: Missing semicolon
int x = 5

// Linter Error 1: Bad constant naming
const badConstant = 10;

// Linter Error 2: Bad function naming
function SCREAMING_FUNCTION() {
    io:println("bad name");
}

// Parser Error 2: Missing type in parameter
function badParams(x) returns int {
    return 42;
}

// Good code mixed in
function goodFunc() returns int {
    int value = 100;
    return value;
}

// Semantic Error: Using undeclared variable
function usesBadVar() {
    io:println(nonExistent);
}

// Linter Error 3: Another bad constant
const anotherBad = 200;
