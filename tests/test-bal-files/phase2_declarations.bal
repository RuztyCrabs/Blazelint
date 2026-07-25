import ballerina/io;

public type Person record {
    int id;
    string name;
    string email?;
};

type Status "OPEN"|"CLOSED"|"PENDING";

public enum Color {
    RED,
    GREEN,
    BLUE
}

configurable int port = 8080;
configurable string host = ?;

public class Counter {
    private int count = 0;

    public function increment() {
        self.count += 1;
    }

    public function get() returns int {
        return self.count;
    }
}

public function main() returns error? {
    Color c = RED;
    int p = port;
    io:println(c);
    io:println(p);
    io:println(host);
    Counter counter = new;
    io:println(counter);
}
