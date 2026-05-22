#include <stdint.h>

int main(void) {
    int64_t total = 0;
    for (int64_t i = 1; i <= 1000000; i++) {
        total += i;
    }
    return (int)(total & 255);
}
