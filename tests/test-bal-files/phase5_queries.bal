import ballerina/io;

public function main() {
    int[] numbers = [1, 2, 3, 4, 5];

    int[] doubled = from int n in numbers
                    where n > 2
                    let int twice = n * 2
                    order by n descending
                    limit 10
                    select twice;
    io:println(doubled);

    int[] joined = from int a in numbers
                   join int b in numbers on a equals b
                   select a;
    io:println(joined);

    var people = table [
        {id: 1, name: "Ann"},
        {id: 2, name: "Bob"}
    ];
    io:println(people);
}
