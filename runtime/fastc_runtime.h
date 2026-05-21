/* FastC Runtime Header */
#ifndef FASTC_RUNTIME_H
#define FASTC_RUNTIME_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>

/* Trap handler - abort on safety violation */
static inline _Noreturn void fc_trap(void) {
    abort();
}

/* Allocator stubs - users may replace */
static inline void* fc_alloc(size_t size, size_t align) {
    (void)align; /* C11 aligned_alloc if needed */
    return malloc(size);
}

static inline void fc_free(void* ptr) {
    free(ptr);
}

/* Stdout helpers: accept const uint8_t* so the prelude can keep its
 * raw(u8) signatures without triggering -Wpointer-sign under -Werror
 * when the underlying libc functions want plain `char*`. */
static inline int fc_puts_u8(const uint8_t* s) {
    return puts((const char*)s);
}

static inline int fc_putchar(int c) {
    return putchar(c);
}

/* Format and write a signed 32-bit integer in base 10. Returns the
 * number of bytes written, mirroring printf's return contract. Used
 * by the prelude's `io::print_int` helper. Manual loop instead of
 * snprintf because the latter pulls in a *lot* of libc surface; this
 * keeps the runtime header free of <inttypes.h> and friends. */
static inline int fc_print_i32(int32_t n) {
    if (n == 0) {
        putchar('0');
        return 1;
    }
    int written = 0;
    if (n < 0) {
        putchar('-');
        written++;
        n = -n;
    }
    char buf[12]; /* INT32_MIN absolute value is 10 digits + sign */
    int len = 0;
    while (n > 0) {
        buf[len++] = (char)('0' + (n % 10));
        n /= 10;
    }
    while (len > 0) {
        putchar(buf[--len]);
        written++;
    }
    return written;
}

/* Memory copy */
static inline void fc_memcpy(void* dst, const void* src, size_t n) {
    unsigned char* d = (unsigned char*)dst;
    const unsigned char* s = (const unsigned char*)src;
    while (n--) {
        *d++ = *s++;
    }
}

/* Unaligned read helpers - use memcpy for safe unaligned access */
static inline uint16_t fc_read_u16_unaligned(const void* ptr) {
    uint16_t val;
    fc_memcpy(&val, ptr, sizeof(val));
    return val;
}

static inline uint32_t fc_read_u32_unaligned(const void* ptr) {
    uint32_t val;
    fc_memcpy(&val, ptr, sizeof(val));
    return val;
}

static inline uint64_t fc_read_u64_unaligned(const void* ptr) {
    uint64_t val;
    fc_memcpy(&val, ptr, sizeof(val));
    return val;
}

/* Unaligned write helpers */
static inline void fc_write_u16_unaligned(void* ptr, uint16_t val) {
    fc_memcpy(ptr, &val, sizeof(val));
}

static inline void fc_write_u32_unaligned(void* ptr, uint32_t val) {
    fc_memcpy(ptr, &val, sizeof(val));
}

static inline void fc_write_u64_unaligned(void* ptr, uint64_t val) {
    fc_memcpy(ptr, &val, sizeof(val));
}

/* Slice type macro */
#define FC_DEFINE_SLICE(T, name) \
    typedef struct { T* data; size_t len; } name

/* Common slice types */
FC_DEFINE_SLICE(uint8_t, fc_slice_uint8_t);
FC_DEFINE_SLICE(int8_t, fc_slice_int8_t);
FC_DEFINE_SLICE(uint16_t, fc_slice_uint16_t);
FC_DEFINE_SLICE(int16_t, fc_slice_int16_t);
FC_DEFINE_SLICE(uint32_t, fc_slice_uint32_t);
FC_DEFINE_SLICE(int32_t, fc_slice_int32_t);
FC_DEFINE_SLICE(uint64_t, fc_slice_uint64_t);
FC_DEFINE_SLICE(int64_t, fc_slice_int64_t);
FC_DEFINE_SLICE(float, fc_slice_float);
FC_DEFINE_SLICE(double, fc_slice_double);

#endif /* FASTC_RUNTIME_H */
