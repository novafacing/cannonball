#include <stdio.h>

const char *x = "hello world";

int main() {
    char z = 0;
    for (int i = 0; i < 11; i++) {
        z = x[i];
    }
}