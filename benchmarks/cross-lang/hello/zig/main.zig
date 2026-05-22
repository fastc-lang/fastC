const std = @import("std");

extern fn puts(s: [*:0]const u8) c_int;

pub fn main() void {
    _ = puts("Hello");
}
