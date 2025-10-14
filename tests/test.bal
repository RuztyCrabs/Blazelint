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
int anotherIdent123 = 456;
const API_KEY = "my-secret-key";
// string x = 'iuygewkfugkeuwgfiuwegftiugwiueghiuwegfiuwghfighweifguhaesIUFHGIO WEHFGIOW8HGIUHWEVIUHG FIUEWHGIUWYHGIUWHEGFIU   WEHGIUWHEGIUWEHGiuwhgiuwHGIUWHEGIUWHGIUWHGIWUAHGiwuehgiwuhgiuwahgiaukheg'
string longLine = "this is a very long line that is longer than 120 
characters just to test the line length rule in the linter, so that it
 will trigger the error and we can see the output of the linter";