import ballerina/io;

// This file is designed to trigger all linting rules in Blazelint.

// RULE: ConstantCaseRule - badConstant should be in SCREAMING_SNAKE_CASE
const badConstant = 100;

// RULE: CamelCaseRule - variable_name should be in camelCase
int variable_name = 10;

// RULE: MissingReturnRule - this function is declared to return int but doesn't
function missingReturnFunction() returns int {
    int x = 5;
    if (x > 0) {
        // Missing return here
    }
    // No return statement at the end
}

// RULE: UnusedVariablesRule - unused_var is never used
function unusedVariablesFunction() {
    int unused_var = 1;
    io:println("This function has an unused variable.");
}

// RULE: MaxFunctionLengthRule - This function will exceed the maximum allowed lines (default 50)
public function longFunctionForLintTest() {
    int line0 = 0;
    int line1 = 1;
    int line2 = 2;
    int line3 = 3;
    int line4 = 4;
    int line5 = 5;
    int line6 = 6;
    int line7 = 7;
    int line8 = 8;
    int line9 = 9;
    int line10 = 10;
    int line11 = 11;
    int line12 = 12;
    int line13 = 13;
    int line14 = 14;
    int line15 = 15;
    int line16 = 16;
    int line17 = 17;
    int line18 = 18;
    int line19 = 19;
    int line20 = 20;
    int line21 = 21;
    int line22 = 22;
    int line23 = 23;
    int line24 = 24;
    int line25 = 25;
    int line26 = 26;
    int line27 = 27;
    int line28 = 28;
    int line29 = 29;
    int line30 = 30;
    int line31 = 31;
    int line32 = 32;
    int line33 = 33;
    int line34 = 34;
    int line35 = 35;
    int line36 = 36;
    int line37 = 37;
    int line38 = 38;
    int line39 = 39;
    int line40 = 40;
    int line41 = 41;
    int line42 = 42;
    int line43 = 43;
    int line44 = 44;
    int line45 = 45;
    int line46 = 46;
    int line47 = 47;
    int line48 = 48;
    int line49 = 49;
    int line50 = 50; // This is the 51st line in the function body, exceeding max_function_length of 50
}

// RULE: LineLengthRule - This line intentionally exceeds the maximum line length of 120 characters to trigger the LineLengthRule.
string another_long_line_for_testing = "This is another intentionally exceedingly long line to ensure that the LineLengthRule is properly triggered and reported by the linter. It goes on and on, far beyond the recommended 120-character limit, just to make sure the detection mechanism works as expected. We need this for robust testing of the code quality checks implemented within the Blazelint system. Without such excessively long lines, we might not truly verify the effectiveness of our line length constraint enforcement. Therefore, its presence here is not an oversight but a deliberate strategy for thorough quality assurance and validation. This meticulous approach guarantees that our linter functions reliably in identifying and flagging code that deviates from established style guidelines. A robust linter contributes significantly to maintaining a consistent and readable codebase across an entire project, improving collaboration and reducing the cognitive load for developers working on the same repository. We are committed to delivering a high-quality static analysis tool that aids developers in creating maintainable and idiomatic code.";


function main() {
    // Call functions to ensure all rules are checked
    int result = missingReturnFunction(); // This will not return a value
    io:println("Result of missingReturnFunction: " + result);

    unusedVariablesFunction();
    longFunctionForLintTest();

    io:println("badConstant: " + badConstant);
    io:println("variable_name: " + variable_name);
    io:println("Another long line length test.");
    io:println(another_long_line_for_testing);
}
