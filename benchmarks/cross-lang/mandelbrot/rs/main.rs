use std::io::{self, Write, BufWriter};

fn main() {
    let width = 800;
    let height = 800;
    let max_iter = 100;
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for y in 0..height {
        for x in 0..width {
            let cx = (x as f64 / width as f64) * 3.5 - 2.5;
            let cy = (y as f64 / height as f64) * 2.0 - 1.0;
            let mut zx = 0.0f64;
            let mut zy = 0.0f64;
            let mut i = 0;
            while i < max_iter {
                let zx2 = zx * zx;
                let zy2 = zy * zy;
                if zx2 + zy2 > 4.0 {
                    break;
                }
                let new_zx = zx2 - zy2 + cx;
                let new_zy = 2.0 * zx * zy + cy;
                zx = new_zx;
                zy = new_zy;
                i += 1;
            }
            out.write_all(if i == max_iter { b"*" } else { b" " }).unwrap();
        }
        out.write_all(b"\n").unwrap();
    }
}
