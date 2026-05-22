fn main() {
    let mut total: i64 = 0;
    for i in 1..=1_000_000i64 {
        total += i;
    }
    std::process::exit((total & 255) as i32);
}
