fn fib(n: i64) -> i64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    let r = fib(40);
    std::process::exit((r & 255) as i32);
}
