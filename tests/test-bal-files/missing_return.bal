function getValue() returns int {
    if (true) {
        return 42;
    }else {
        // Missing return
    }
}

function anotherGetValue() returns int {
    if (true) {
        return 42;
    } else {
        return 0;
    }
}

function noReturn() {

}

function complexGetValue(int a) returns int {
    if (a > 10) {
        return 1;
    } else if (a > 5) {
        return 2;
    }else {
        return 3;
    }
}
