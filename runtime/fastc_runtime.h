/* FastC Runtime Header
 *
 * Portability: this header is C11 + a small slice of POSIX (`unistd.h`'s
 * `access`, `sys/stat.h`'s `stat`, `time.h`'s `time`, `stdlib.h`'s
 * `getenv`). All eight v1.9 cross-compile targets (aarch64/x86_64 ×
 * linux-musl/linux-gnu, aarch64/x86_64-macos, wasm32-wasi,
 * riscv64-linux-musl) provide that surface via their bundled libc.
 *
 * The Windows-msvc target is deliberately *not* in the v1.9 set — its
 * different ABI (no POSIX `access`, `_stat` instead of `stat`) needs
 * its own `#ifdef _WIN32` wrappers and a real user request. See
 * docs/cross-compile.md for the current matrix.
 */
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

/* Command-line argv access.
 *
 * fastC's `fn main() -> i32` takes no arguments. To expose argv to user
 * code, the emit pass renames the user's body to `fc_user_main` and
 * generates a wrapper `int main(int argc, char** argv)` that calls
 * `fc_args_init` before invoking it. The user-facing surface lives in
 * `mod cli` in the prelude.
 *
 * `_fastc_argv` is a `char**` pointer to libc's argv array — its
 * lifetime matches the process, so we can stash the pointer for the
 * full run without copying. The lookup is bounds-checked to return
 * NULL for out-of-range indices, which `mod cli` translates to
 * "missing argument".
 */
static int _fastc_argc = 0;
static char** _fastc_argv = (char**)0;

static inline void fc_args_init(int argc, char** argv) {
    _fastc_argc = argc;
    _fastc_argv = argv;
}

static inline int32_t fc_args_count(void) {
    return (int32_t)_fastc_argc;
}

static inline const uint8_t* fc_args_at(int32_t i) {
    if (i < 0 || i >= _fastc_argc || _fastc_argv == (char**)0) {
        return (const uint8_t*)0;
    }
    return (const uint8_t*)_fastc_argv[i];
}

/* Advance a `const uint8_t*` by one byte. fastC's `raw(u8)` type
 * doesn't expose pointer-add directly today; `mod cli` uses this
 * helper to walk past a flag prefix to land on its value. */
static inline const uint8_t* fc_raw_step(const uint8_t* p) {
    if (p == (const uint8_t*)0) return p;
    return p + 1;
}

/* Null-pointer helpers. fastC has no `cast(raw(u8), 0)` syntax yet
 * (the type system rejects integer→pointer casts), so callers that
 * need to spell "NULL" or null-check a pointer go through these. */
static inline const uint8_t* fc_raw_null(void) {
    return (const uint8_t*)0;
}

static inline bool fc_raw_is_null(const uint8_t* p) {
    return p == (const uint8_t*)0;
}

/* TCP socket primitives for `mod http`.
 *
 * Thin POSIX-socket wrappers used by the stage-1.8 `fastc-core/http`
 * launch-set preview. The fastC bindings live in `mod http` and gate
 * every call on a `CapNetConnect` capability so the type system
 * tracks "this code reaches the network."
 *
 * Portability: POSIX `socket` / `connect` / `send` / `recv` /
 * `close` are available on every v1.9 cross-compile target EXCEPT
 * wasm32-wasi. wasi-libc ships `<sys/socket.h>` (since 2024) but the
 * actual `socket()` / `connect()` symbols aren't linkable — WASI
 * Preview 1 has no synchronous BSD-socket equivalent and Preview 2's
 * `wasi:sockets` is a different API that needs a separate binding.
 * For now we gate the body on `!__wasi__`; the fastC `mod http`
 * binding compiles fine for WASI but calling it links against weak
 * stubs (defined further down) that return -1.
 */
#ifndef __wasi__
#include <sys/socket.h>
#include <netinet/in.h>
#include <netdb.h>
#include <string.h>
#include <errno.h>
#endif
#include <unistd.h>

/* Open a TCP connection to `host:port`. Returns the socket fd, or
 * -1 on resolve/connect failure. `host` is a null-terminated cstr
 * (e.g. "example.com" or "127.0.0.1"); `port` is the IP port.
 *
 * On wasm32-wasi this is a stub that always returns -1 (see the
 * portability note above). User code can still link, and the
 * fastC `http::get_status` binding still type-checks; the call
 * just fails at runtime with status -1. */
#ifndef __wasi__
static inline int32_t fc_net_connect_tcp(const uint8_t* host, int32_t port) {
    struct addrinfo hints;
    struct addrinfo* res = (struct addrinfo*)0;
    struct addrinfo* p;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    char port_str[8];
    snprintf(port_str, sizeof(port_str), "%d", (int)port);
    if (getaddrinfo((const char*)host, port_str, &hints, &res) != 0) {
        return -1;
    }
    int sock = -1;
    for (p = res; p != (struct addrinfo*)0; p = p->ai_next) {
        sock = socket(p->ai_family, p->ai_socktype, p->ai_protocol);
        if (sock < 0) continue;
        if (connect(sock, p->ai_addr, p->ai_addrlen) == 0) break;
        close(sock);
        sock = -1;
    }
    freeaddrinfo(res);
    return (int32_t)sock;
}

static inline int32_t fc_net_send(int32_t fd, const uint8_t* buf, int32_t len) {
    if (fd < 0 || len <= 0) return -1;
    ssize_t n = send((int)fd, buf, (size_t)len, 0);
    return (int32_t)n;
}

static inline int32_t fc_net_recv(int32_t fd, uint8_t* buf, int32_t cap) {
    if (fd < 0 || cap <= 0) return -1;
    ssize_t n = recv((int)fd, buf, (size_t)cap, 0);
    return (int32_t)n;
}

static inline void fc_net_close(int32_t fd) {
    if (fd >= 0) close((int)fd);
}
#else
/* wasm32-wasi stubs: same signatures, always-fail behaviour. The
 * `(void)<arg>` casts silence -Wunused-parameter under -Werror. */
static inline int32_t fc_net_connect_tcp(const uint8_t* host, int32_t port) {
    (void)host; (void)port; return -1;
}
static inline int32_t fc_net_send(int32_t fd, const uint8_t* buf, int32_t len) {
    (void)fd; (void)buf; (void)len; return -1;
}
static inline int32_t fc_net_recv(int32_t fd, uint8_t* buf, int32_t cap) {
    (void)fd; (void)buf; (void)cap; return -1;
}
static inline void fc_net_close(int32_t fd) {
    (void)fd;
}
#endif

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
