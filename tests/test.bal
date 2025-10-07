// This is a test comment
var myInt = 123;

function calculate(a: float, b: float) returns float {
    /* This is a 
    Multi - line
    * comment */
    if (a > b || a != b) {
        return (a * b) / (a + b); // Complex expression
    } else {
        panic error("Invalid operation!");
    }
}

string message = "Hello, Ballerina!";
boolean isDone = true;
float myFloat = 0.005e+2;
int another_ident_123 = 456;
