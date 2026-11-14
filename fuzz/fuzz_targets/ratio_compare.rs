#![no_main]
use core::cmp::Ordering;
use dimvexa_core::Ratio;
use libfuzzer_sys::fuzz_target;

// Equality and order must be total on equal exponents, agree with each other, be
// antisymmetric and never panic, whatever the 128-bit inputs.
fuzz_target!(|data: (i128, i128, i128, i128, i16, i16)| {
    let (an, ad, bn, bd, ta, tb) = data;
    let (Ok(a), Ok(b)) = (Ratio::new(an, ad), Ratio::new(bn, bd)) else {
        return;
    };
    let (a, b) = (
        a.with_exponents(i32::from(ta), 0),
        b.with_exponents(i32::from(tb), 0),
    );
    let _ = a.is_integer();
    let eq = a.checked_eq(b).expect("equality is total without pi");
    assert_eq!(eq, b.checked_eq(a).expect("symmetric"));
    if a == b {
        assert!(eq);
    }
    if ta == tb {
        let ab = a.checked_cmp(b).expect("same exponents always compare");
        let ba = b.checked_cmp(a).expect("same exponents always compare");
        assert_eq!(ab, ba.reverse());
        assert_eq!(eq, ab == Ordering::Equal);
    }
});
