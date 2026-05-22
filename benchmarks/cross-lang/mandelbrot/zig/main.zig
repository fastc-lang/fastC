extern fn putchar(c: c_int) c_int;

pub fn main() void {
    const width: i32 = 800;
    const height: i32 = 800;
    const max_iter: i32 = 100;

    var y: i32 = 0;
    while (y < height) : (y += 1) {
        var x: i32 = 0;
        while (x < width) : (x += 1) {
            const cx: f64 = (@as(f64, @floatFromInt(x)) / @as(f64, @floatFromInt(width))) * 3.5 - 2.5;
            const cy: f64 = (@as(f64, @floatFromInt(y)) / @as(f64, @floatFromInt(height))) * 2.0 - 1.0;
            var zx: f64 = 0.0;
            var zy: f64 = 0.0;
            var i: i32 = 0;
            while (i < max_iter) : (i += 1) {
                const zx2 = zx * zx;
                const zy2 = zy * zy;
                if (zx2 + zy2 > 4.0) break;
                const new_zx = zx2 - zy2 + cx;
                const new_zy = 2.0 * zx * zy + cy;
                zx = new_zx;
                zy = new_zy;
            }
            _ = putchar(if (i == max_iter) '*' else ' ');
        }
        _ = putchar('\n');
    }
}
