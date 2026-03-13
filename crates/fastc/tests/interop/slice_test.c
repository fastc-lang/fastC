/* Slice ABI verification test */
#include <stddef.h>
#include <stdint.h>
#include <assert.h>

/* Expected slice layout from FastC */
typedef struct {
    int32_t* data;
    size_t len;
} fc_slice_int32_t;

int main(void) {
    /* Verify slice layout: { T* data; size_t len; } */
    assert(offsetof(fc_slice_int32_t, data) == 0);
    assert(offsetof(fc_slice_int32_t, len) == sizeof(int32_t*));
    assert(sizeof(fc_slice_int32_t) == sizeof(void*) + sizeof(size_t));

    /* Test slice usage */
    int32_t arr[] = { 1, 2, 3, 4, 5 };
    fc_slice_int32_t slice = { .data = arr, .len = 5 };

    assert(slice.len == 5);
    assert(slice.data[0] == 1);
    assert(slice.data[4] == 5);

    return 0;
}
