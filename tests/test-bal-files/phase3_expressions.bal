import ballerina/io;

function readData() returns int|error {
    return 5;
}

public function main() returns error? {
    int value = check readData();
    int safe = checkpanic readData();
    io:println(value);
    io:println(safe);

    var double = x => x * 2;
    var add = (int a, int b) => a + b;
    var greet = function(string name) returns string {
        return "Hi";
    };
    io:println(double);
    io:println(add);
    io:println(greet);

    int computed = let int a = 10, int b = 20 in a + b;
    io:println(computed);

    typeof value;
}
