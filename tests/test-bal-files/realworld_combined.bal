import ballerina/io;

// A broad, idiomatic Ballerina program exercising features from every phase of
// the grammar expansion together: records, enums, type aliases, configurable
// variables, classes, error handling (check/error), match with guards and
// binding/list patterns, query expressions, do/on-fail, and the `is` type test.
// It must parse and analyze without errors.

public type Employee record {|
    int id;
    string name;
    string department;
    decimal salary;
|};

public enum Department {
    ENGINEERING,
    SALES,
    HR
}

type Result Employee|error;

configurable decimal bonusRate = 0.1;

function applyBonus(decimal salary) returns decimal {
    return salary + (salary * bonusRate);
}

public class Payroll {
    private Employee[] employees;

    public function init(Employee[] employees) {
        self.employees = employees;
    }

    public function totalPayroll() returns decimal {
        decimal total = 0;
        foreach Employee e in self.employees {
            total += applyBonus(e.salary);
        }
        return total;
    }

    public function highEarners(decimal threshold) returns Employee[] {
        return from Employee e in self.employees
            where e.salary > threshold
            order by e.salary descending
            select e;
    }
}

function findEmployee(Employee[] emps, int id) returns Result {
    foreach Employee e in emps {
        if (e.id == id) {
            return e;
        }
    }
    return error("not found");
}

public function main() returns error? {
    Employee[] emps = [
        {id: 1, name: "Ann", department: "ENG", salary: 100},
        {id: 2, name: "Bob", department: "SALES", salary: 80}
    ];

    Payroll payroll = new (emps);
    decimal total = payroll.totalPayroll();
    io:println(total);

    Result r = findEmployee(emps, 1);
    match r {
        var emp if emp is Employee => {
            io:println(emp.name);
        }
        _ => {
            io:println("error");
        }
    }

    Employee found = check findEmployee(emps, 2);
    io:println(found.name);

    var names = from Employee e in emps
        select e.name;
    io:println(names);

    do {
        Employee e = check findEmployee(emps, 3);
        io:println(e);
    } on fail error err {
        io:println(err);
    }
}
