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

/* Format and write a signed 64-bit integer in base 10. Same shape
 * as fc_print_i32 but wide enough for sums that exceed INT32_MAX
 * (e.g. the T4 / T5 cross-language overflow benchmark). */
static inline int fc_print_i64(int64_t n) {
    if (n == 0) {
        putchar('0');
        return 1;
    }
    int written = 0;
    int64_t v = n;
    if (v < 0) {
        putchar('-');
        written++;
        v = -v;
    }
    char buf[24]; /* INT64_MIN absolute value is 19 digits + sign */
    int len = 0;
    while (v > 0) {
        buf[len++] = (char)('0' + (v % 10));
        v /= 10;
    }
    while (len > 0) {
        putchar(buf[--len]);
        written++;
    }
    return written;
}

/* Read a signed 32-bit integer from stdin via libc scanf. Returns 0
 * on parse failure (which is indistinguishable from a successful "0"
 * read — callers needing finer granularity should use the
 * `fc_read_i32_ok` variant below). Closes the v1 gap that prevented
 * the benchmark's T2 (is_prime) and T3 (json_token) from being
 * solvable in fastC at all. Cap-free because stdin is treated as the
 * same kind of ambient resource as stdout via `io::println`. A
 * follow-up sub-slice will introduce `CapStdinRead` if the
 * capability story tightens. */
static inline int32_t fc_read_i32(void) {
    int32_t n = 0;
    if (scanf("%d", &n) != 1) return 0;
    return n;
}

/* Read a signed 64-bit integer from stdin. Same cap-free policy as
 * fc_read_i32. */
static inline int64_t fc_read_i64(void) {
    long long n = 0;
    if (scanf("%lld", &n) != 1) return 0;
    return (int64_t)n;
}

/* Current Unix epoch in seconds. Wraps libc `time(NULL)` so the
 * fastC `time::now` binding doesn't need to construct a NULL raw
 * pointer (which the type system doesn't expose cleanly today).
 * Returns int64_t for cross-platform stability — libc `time_t` is
 * platform-defined and we widen here. Used by `mod time` in the
 * prelude; the capability check happens at the fastC level so this
 * helper itself is unprivileged. */
#include <time.h>
static inline int64_t fc_time_now(void) {
    return (int64_t)time(NULL);
}

/* Look up an environment variable by null-terminated key. Returns
 * the raw byte pointer libc gives us (which may be NULL when the
 * variable isn't set). Used by `mod env::get`; the cap check
 * happens at the fastC level so this helper itself is unprivileged.
 *
 * `getenv` returns `char*`; we cast to `const uint8_t*` to match
 * the fastC `raw(u8)` surface and avoid -Wpointer-sign noise. */
static inline const uint8_t* fc_env_get(const uint8_t* key) {
    return (const uint8_t*)getenv((const char*)key);
}

/* File existence check. Returns 1 if the path is reachable via
 * libc `access(..., F_OK)`, 0 otherwise. Used by `mod fs::exists`;
 * the cap check happens at the fastC level. */
#include <unistd.h>
static inline int32_t fc_fs_exists(const uint8_t* path) {
    return access((const char*)path, F_OK) == 0 ? 1 : 0;
}

/* Byte-size of a regular file. Returns -1 if the path can't be
 * stat'd or isn't a regular file. Used by `mod fs::size_bytes`;
 * widens to int64_t so the fastC binding doesn't have to worry
 * about libc off_t portability. */
#include <sys/stat.h>
static inline int64_t fc_fs_size_bytes(const uint8_t* path) {
    struct stat st;
    if (stat((const char*)path, &st) != 0) return -1;
    /* Only count regular files; directories/sockets/etc are not
     * what callers asking for a byte count want. */
    if ((st.st_mode & S_IFMT) != S_IFREG) return -1;
    return (int64_t)st.st_size;
}

/* Seedable linear-congruential PRNG. Single global state so the
 * fastC binding doesn't have to thread a struct around. Output is
 * uint32; callers can narrow as needed.
 *
 * Constants are Numerical Recipes' values (a=1664525, c=1013904223),
 * which give a full-period 2^32 cycle. This is a deliberate v1
 * choice — predictable, no platform libc/jrand dependency, easy to
 * golden-test. A cryptographically-strong RNG follows once `mod
 * crypto` lands in fastc-core. The capability check is at the
 * fastC level so this helper is unprivileged. */
static uint32_t fc_rand_state = 1;

static inline void fc_rand_seed(uint32_t seed) {
    fc_rand_state = seed;
}

static inline uint32_t fc_rand_u32(void) {
    fc_rand_state = fc_rand_state * 1664525u + 1013904223u;
    return fc_rand_state;
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

/* Test-only accumulator used by examples that need a side-effect
 * sink before closures with captured state exist. Three helpers:
 *   - fc_test_acc_reset(): clear the slot
 *   - fc_test_acc_add(int32_t): add to the slot
 *   - fc_test_acc_get(): read the slot back
 * The state is a file-static int32_t so the helpers are self-contained.
 * Removed once closures land. */
static int32_t fc_test_accumulator_slot = 0;

static inline void fc_test_acc_reset(void) {
    fc_test_accumulator_slot = 0;
}

static inline void fc_test_acc_add(int32_t x) {
    fc_test_accumulator_slot += x;
}

static inline int32_t fc_test_acc_get(void) {
    return fc_test_accumulator_slot;
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
