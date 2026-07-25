import ballerina/io;

function describe(int code) returns string {
    match code {
        1 => {
            return "one";
        }
        2 | 3 => {
            return "few";
        }
        var other => {
            return "many";
        }
    }
}

public function main() returns error? {
    string s = describe(2);
    io:println(s);

    do {
        int x = check readValue();
        io:println(x);
    } on fail error e {
        io:println(e);
    }

    lock {
        int counter = 5;
        io:println(counter);
    }

    transaction {
        int y = 1;
        io:println(y);
        check commit;
    }

    int[] items = [1, 2, 3];
    match items {
        [var first, var second] => {
            io:println(first);
            io:println(second);
        }
        _ => {
        }
    }
}

function readValue() returns int|error {
    return 10;
}
