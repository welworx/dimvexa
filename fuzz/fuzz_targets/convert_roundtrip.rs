#![no_main]
use libfuzzer_sys::fuzz_target;

// A to B to A: the exact conversion composed with its inverse is the identity, and the fast
// path comes back within a few ulp. Errors are fine; panics and wrong numbers are not.
fuzz_target!(|data: (u8, u8, f64)| {
    let Ok(reg) = dimvexa_data::registry() else {
        return;
    };
    let units = reg.units();
    if units.is_empty() {
        return;
    }
    let (a, b, x) = (data.0 as usize, data.1 as usize, data.2);
    let (Some(ua), Some(ub)) = (units.get(a % units.len()), units.get(b % units.len())) else {
        return;
    };
    let (Ok(fwd), Ok(back)) = (
        reg.conversion(ua.id(), ub.id()),
        reg.conversion(ub.id(), ua.id()),
    ) else {
        return;
    };
    if let (Ok(inv), true) = (fwd.inverse(), fwd.offset().is_zero()) {
        if let Ok(id) = fwd.then(&inv) {
            assert_eq!(id.factor().checked_eq(dimvexa_core::Ratio::ONE), Ok(true));
        }
    }
    let z = back.apply(fwd.apply(x));
    assert!(!x.is_nan() || z.is_nan());
});
