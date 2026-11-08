//! Prints the Rust fast-path result bits for the smoke test: `<from> <to> <hex bits...>`.
use dimvexa::registry;

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(1000);
    let mut x: u32 = 12345;
    let xs: Vec<f64> = (0..n)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f64::from(x) / 4_294_967_296.0 * 2000.0 - 500.0
        })
        .collect();
    let reg = registry().expect("table");
    for (f, t) in [("m/s", "km/h"), ("psi", "kPa"), ("degC", "degF")] {
        let c = reg.conversion(f, t).expect("conversion");
        let mut v = xs.clone();
        c.apply_in_place(&mut v);
        let hex: Vec<String> = v.iter().map(|y| format!("{:x}", y.to_bits())).collect();
        println!("{f} {t} {}", hex.join(" "));
    }
}
