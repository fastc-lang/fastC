/* Struct layout verification test */
#include <stddef.h>
#include <stdint.h>
#include <assert.h>

/* Expected struct layout from FastC @repr(C) struct */
typedef struct Point {
    int32_t x;
    int32_t y;
} Point;

int main(void) {
    /* Verify struct layout matches C expectations */
    assert(offsetof(Point, x) == 0);
    assert(offsetof(Point, y) == sizeof(int32_t));
    assert(sizeof(Point) == 2 * sizeof(int32_t));

    /* Verify field sizes */
    Point p = { .x = 42, .y = 100 };
    assert(p.x == 42);
    assert(p.y == 100);

    return 0;
}
