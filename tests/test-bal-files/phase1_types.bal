import ballerina/io;

// Exercises the Phase 1 type-descriptor grammar expansion: tuples, inline
// records, generics, intersections, optionals, unions, qualified names, and
// named/predeclared types. Must parse and analyze without errors.

public function main() returns int|error {
    int[] nums = [1, 2, 3];
    [int, string] pair = [1, "hello"];
    map<int> counts = {a: 1, b: 2};
    record {int id; string name;} person = {id: 1, name: "Sam"};
    record {|int id; string label?;|} closed = {id: 2};
    stream<int> numbers = nums;
    readonly & int config = 5;
    int? maybe = 7;
    json data = counts;
    string|int mixed = "either";

    foreach int n in nums {
        io:println(n);
    }

    io:println(pair);
    io:println(counts);
    io:println(person);
    io:println(closed);
    io:println(numbers);
    io:println(config);
    io:println(maybe);
    io:println(data);
    io:println(mixed);
    return 42;
}

function transform(function (int) returns int mapper, int[] values) returns int {
    io:println(mapper);
    int total = 0;
    foreach int v in values {
        total += v;
    }
    return total;
}
