const std = @import("std");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    var client = std.http.Client{ .allocator = allocator };
    defer client.deinit();

    var server_header_buffer: [1024]u8 = undefined;
    var req = try client.open(.GET, try std.Uri.parse("http://example.com"), .{
        .server_header_buffer = &server_header_buffer,
    });
    defer req.deinit();

    try req.send();
    try req.finish();
    try req.wait();
    std.debug.print("status: {d}\n", .{@intFromEnum(req.response.status)});
}
