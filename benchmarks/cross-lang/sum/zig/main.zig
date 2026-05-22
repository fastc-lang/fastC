pub fn main() u8 {
    var total: i64 = 0;
    var i: i64 = 1;
    while (i <= 1_000_000) : (i += 1) {
        total += i;
    }
    return @intCast(total & 255);
}
