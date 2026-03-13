/* Enum representation verification test */
#include <stddef.h>
#include <stdint.h>
#include <assert.h>

/* Expected enum representation from FastC */
typedef enum Color {
    Color_Red,
    Color_Green,
    Color_Blue
} Color;

int main(void) {
    /* Verify enum values */
    assert(Color_Red == 0);
    assert(Color_Green == 1);
    assert(Color_Blue == 2);

    /* Verify enum size (default i32 representation) */
    assert(sizeof(Color) == sizeof(int));

    /* Test enum assignment */
    Color c = Color_Green;
    assert(c == 1);

    return 0;
}
