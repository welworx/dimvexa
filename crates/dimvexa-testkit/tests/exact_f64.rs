// SPDX-License-Identifier: MIT OR Apache-2.0
//! Proves that `Conversion::apply_exact_f64` returns the correctly rounded result.
//!
//! "Correctly rounded" means: no other `f64` is closer to the exact rational result. The test
//! checks that directly. For many conversions and inputs it compares the distance from the
//! exact value to the returned `f64` with the distance to its two neighbours (all as exact
//! rationals), and on an exact tie it requires the even mantissa.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::panic,
    clippy::many_single_char_names,
    clippy::doc_markdown
)]

use core::cmp::Ordering;

use dimvexa::{Conversion, Ratio, Shape, registry};
use dimvexa_testkit::generators::{edge_f64, spread_f64};
use dimvexa_testkit::rng::Rng;

fn next_up(x: f64) -> f64 {
    if x == 0.0 {
        return f64::from_bits(1);
    }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}

fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

fn abs_diff(a: Ratio, b: Ratio) -> Option<Ratio> {
    let d = a.checked_sub(b).ok()?;
    if d.signum() < 0 {
        d.checked_neg().ok()
    } else {
        Some(d)
    }
}

/// `Some(true)` if `candidate` is at least as close to `exact` as `other`, `None` if the
/// rational arithmetic overflows (the comparison is then skipped, and counted).
fn at_least_as_close(exact: Ratio, candidate: f64, other: f64) -> Option<Ordering> {
    let dc = abs_diff(exact, Ratio::from_f64(candidate).ok()?)?;
    let dn = abs_diff(exact, Ratio::from_f64(other).ok()?)?;
    dc.checked_cmp(dn).ok()
}

#[derive(Default, Debug)]
struct Tally {
    proved: u64,
    ties: u64,
    skipped: u64,
}

fn prove_one(conv: &Conversion, x: f64, label: &str, t: &mut Tally) {
    let Ok(xr) = Ratio::from_f64(x) else {
        t.skipped += 1;
        return;
    };
    let Ok(exact) = conv.apply_exact(xr) else {
        t.skipped += 1;
        return;
    };
    let got = conv.apply_exact_f64(x).unwrap();
    assert!(got.is_finite(), "{label}: {x:e}");
    for (name, other) in [("below", next_down(got)), ("above", next_up(got))] {
        match at_least_as_close(exact, got, other) {
            None => {
                t.skipped += 1;
                return;
            }
            Some(Ordering::Greater) => {
                panic!(
                    "{label}: x = {x:e}: neighbour {name} ({other:e}) is closer to the exact result than {got:e}"
                )
            }
            Some(Ordering::Equal) => {
                // exact tie: the even mantissa must win
                assert_eq!(
                    got.to_bits() & 1,
                    0,
                    "{label}: tie at x = {x:e} not resolved to even"
                );
                t.ties += 1;
            }
            Some(Ordering::Less) => {}
        }
    }
    t.proved += 1;
}

#[test]
fn exact_f64_is_the_nearest_f64_to_the_exact_result() {
    let reg = registry().unwrap();
    let pairs = [
        ("m/s", "km/h"),
        ("km/h", "m/s"),
        ("psi", "Pa"),
        ("Pa", "psi"),
        ("lb", "kg"),
        ("kg", "lb"),
        ("Btu_IT", "kWh"),
        ("hp_mech", "kW"),
        ("min", "h"),
        ("degF", "degC"),
        ("degC", "degF"),
        ("K", "degC"),
        ("degC", "K"),
        ("degR", "degC"),
        ("degC", "degR"),
        ("degF", "K"),
        ("K", "degF"),
    ];
    let mut rng = Rng::new(0x00ac_c07a_2026_1107);
    let mut tally = Tally::default();
    for (a, b) in pairs {
        let conv = reg.conversion(a, b).unwrap();
        let label = format!("{a} -> {b}");
        let mut xs = edge_f64();
        xs.extend((0..4000).map(|_| spread_f64(&mut rng, -30, 40)));
        // inputs on the decimal grid users really type
        xs.extend((0..2000).map(|_| {
            f64::from(i32::try_from(rng.range_i64(-5_000_000, 5_000_000)).unwrap()) / 100.0
        }));
        for x in xs {
            if x.is_finite() {
                prove_one(&conv, x, &label, &mut tally);
            }
        }
    }
    assert!(tally.proved > 60_000, "{tally:?}");
    assert!(
        tally.skipped * 5 < tally.proved,
        "too many skipped: {tally:?}"
    );
}

#[test]
fn exact_ties_resolve_to_even() {
    // 3/2 * (2^52 + 1) = 6755399441055745.5 lies exactly halfway between two f64 values
    // (the spacing is 1 in [2^52, 2^53)); round half to even picks ...746.
    let conv = Conversion::linear(Ratio::new(3, 2).unwrap()).unwrap();
    let x = 4_503_599_627_370_497.0_f64; // 2^52 + 1, exactly representable
    let exact = Ratio::new(3, 2)
        .unwrap()
        .checked_mul(Ratio::from_f64(x).unwrap())
        .unwrap();
    assert_eq!(exact, Ratio::new(13_510_798_882_111_491, 2).unwrap());
    assert_eq!(conv.apply_exact_f64(x).unwrap(), 6_755_399_441_055_746.0);
    // and 3/2 * (2^52 + 3) = 6755399441055748.5 ties down to the even ...748
    let x = 4_503_599_627_370_499.0_f64;
    assert_eq!(conv.apply_exact_f64(x).unwrap(), 6_755_399_441_055_748.0);
}

#[test]
fn known_values() {
    let reg = registry().unwrap();
    let c = reg.conversion("psi", "Pa").unwrap();
    // 8896443230521/1290320000 rounded once.
    assert_eq!(
        c.apply_exact_f64(1.0).unwrap(),
        Ratio::new(8_896_443_230_521, 1_290_320_000)
            .unwrap()
            .to_f64()
            .unwrap()
    );
    let c = reg.conversion("degF", "degC").unwrap();
    assert_eq!(
        c.apply_exact_f64(32.0).unwrap().to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(c.apply_exact_f64(212.0).unwrap(), 100.0);
    assert_eq!(c.apply_exact_f64(-40.0).unwrap(), -40.0);
    let c = reg.conversion("degC", "K").unwrap();
    assert_eq!(c.apply_exact_f64(0.0).unwrap(), 273.15);
    assert!(c.apply_exact_f64(-273.15).unwrap().abs() < 1e-13);
}

#[test]
fn special_values_are_defined() {
    let reg = registry().unwrap();
    let lin = reg.conversion("km", "m").unwrap();
    let aff = reg.conversion("degF", "degC").unwrap();
    for c in [&lin, &aff] {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let e = c.apply_exact_f64(bad).unwrap_err();
            assert_eq!(e.code(), "invalid_number", "{bad}");
        }
    }
    // signed zero: linear keeps the IEEE sign (as the fast path does), affine exact zero is +0
    assert_eq!(
        lin.apply_exact_f64(-0.0).unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(
        lin.apply_exact_f64(0.0).unwrap().to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(
        aff.apply_exact_f64(32.0).unwrap().to_bits(),
        0.0_f64.to_bits()
    );
    // overflow and subnormal results are errors, not infinities or approximations
    assert_eq!(
        lin.apply_exact_f64(f64::MAX).unwrap_err().code(),
        "overflow"
    );
    let mm = reg.conversion("m", "km").unwrap();
    assert_eq!(
        mm.apply_exact_f64(f64::from_bits(1)).unwrap_err().code(),
        "overflow"
    );
    assert_eq!(
        mm.apply_exact_f64(f64::MIN_POSITIVE).unwrap_err().code(),
        "overflow"
    );
    // extreme but representable inputs work for linear conversions (exponent scaling)
    assert_eq!(lin.apply_exact_f64(1e300).unwrap(), 1e303);
    assert_eq!(lin.apply_exact_f64(-1e-300).unwrap(), -1e-297);
    // an affine conversion of an input the exact arithmetic cannot hold is an error
    assert_eq!(aff.apply_exact_f64(1e300).unwrap_err().code(), "overflow");
    assert_eq!(aff.shape(), Shape::Affine);
    // subnormal input, tiny linear factor: result stays normal -> still correctly rounded
    let big = reg.conversion("EJ", "J").unwrap();
    let y = big.apply_exact_f64(f64::from_bits(1)).unwrap();
    assert!(y > 4.0e-306 && y < 5.0e-306, "{y:e}");
}

/// Independent oracle for factors with a power of pi or of ten (ADR-018): each row is
/// `(input bits, expected output bits)`, computed outside this code base with Python's `decimal`
/// at 140 digits (pi from Machin's formula, the decimal definitions of `eV` and `Da`) and
/// rounded to the nearest `f64` by `float()`, which is correctly rounded.
const ORACLE: &[(&str, &[(u64, u64)])] = &[
    (
        "deg_to_rad",
        &[
            (0x3ff0_0000_0000_0000, 0x3f91_df46_a252_9d39),
            (0x3fe0_0000_0000_0000, 0x3f81_df46_a252_9d39),
            (0x4056_8000_0000_0000, 0x3ff9_21fb_5444_2d18),
            (0x4066_8000_0000_0000, 0x4009_21fb_5444_2d18),
            (0x4076_8000_0000_0000, 0x4019_21fb_5444_2d18),
            (0x3f50_624d_d2f1_a9fc, 0x3ef2_4d15_2119_3474),
            (0x412e_8480_0000_0000, 0x40d1_0b52_b8a5_9179),
            (0x40fe_240c_9fbe_76c9, 0x40a0_d574_7496_5437),
            (0x3fb9_9999_9999_999a, 0x3f5c_9871_03b7_61f5),
            (0x4008_0000_0000_0000, 0x3faa_cee9_f37b_ebd6),
            (0xc1ca_aff6_ac85_a4ad, 0xc16d_cf60_65e9_2609),
            (0xc06e_1c94_3f41_c961, 0xc010_d148_61f6_7b7d),
            (0x4386_25a8_94c1_8421, 0x4328_bd11_c118_15d6),
            (0xbc58_4c6b_2900_2f4e, 0xbbfb_2446_35f3_aba0),
            (0xbdd8_165e_4204_a614, 0xbd7a_e7e6_3e18_4bc5),
            (0xbd69_0e4c_a7e1_8d38, 0xbd0b_fcd7_5b85_03c9),
            (0x4323_9d39_ea19_7c3f, 0x42c5_e8c3_76c4_684e),
            (0xbcda_68a9_cd87_706d, 0xbc7d_7fbb_bed3_6d7c),
            (0xbd03_30cb_5274_7098, 0xbca5_6fa4_d193_9888),
            (0x4195_b0f0_47be_2b84, 0x4138_3ab1_234c_b366),
            (0xbf38_78cd_d392_4613, 0xbedb_55da_6e4a_2e5b),
            (0xc1fb_c1b1_d843_5add, 0xc19f_0123_1d80_4791),
            (0x3da0_8580_4c73_bb20, 0x3d42_7465_ed12_d3dd),
            (0xbdf1_2a3f_ecb7_5197, 0xbd93_2c6c_8d72_15c4),
            (0x3e36_8b67_c70c_32de, 0x3dd9_2eb8_c081_19a2),
            (0x415a_ceca_f865_a989, 0x40fd_f1d0_2d82_3e19),
            (0xbd9c_505c_342e_e0b1, 0xbd3f_a07e_fb5c_5807),
            (0x410c_1a81_271b_5db8, 0x40af_6456_b2b0_a3c0),
            (0xbf83_a378_8b13_f1a8, 0xbf25_efbd_2643_8276),
            (0x40f0_753e_3f7d_4a28, 0x4092_623c_def2_67a3),
            (0x4056_5f5d_d95f_f3c6, 0x3ff8_fd87_a72f_4e2d),
            (0xc250_0535_6dc2_b6af, 0xc1f1_e518_1699_e135),
            (0xc25b_7f4d_ecc2_e622, 0xc1fe_b6fa_7d8e_cc60),
            (0x3eb3_43a4_4313_4338, 0x3e55_84b2_540a_5076),
            (0xc082_6649_2672_a3e0, 0xc024_8d70_8e8c_59c9),
            (0x3bf4_ab02_0375_3133, 0x3b97_161c_cd93_b0aa),
            (0xc10f_bb88_ab86_217c, 0xc0b1_b909_86bc_d814),
            (0xbd5f_1161_aa0f_ccc6, 0xbd01_5a01_968f_5a83),
            (0x3fab_db8e_8f43_03cd, 0x3f4f_1e06_8652_7737),
            (0xbcc7_e392_049e_f31c, 0xbc6a_af28_5e03_9d5b),
        ],
    ),
    (
        "rad_to_deg",
        &[
            (0x3ff0_0000_0000_0000, 0x404c_a5dc_1a63_c1f8),
            (0x3fe0_0000_0000_0000, 0x403c_a5dc_1a63_c1f8),
            (0x4056_8000_0000_0000, 0x40b4_249e_c28e_2462),
            (0x4066_8000_0000_0000, 0x40c4_249e_c28e_2462),
            (0x4076_8000_0000_0000, 0x40d4_249e_c28e_2462),
            (0x3f50_624d_d2f1_a9fc, 0x3fad_55df_566a_3f75),
            (0x412e_8480_0000_0000, 0x418b_521d_1c1a_cae7),
            (0x40fe_240c_9fbe_76c9, 0x415a_fbc4_3d90_60bd),
            (0x3fb9_9999_9999_999a, 0x4016_eb16_7b83_0193),
            (0x4008_0000_0000_0000, 0x4065_7c65_13ca_d17a),
            (0xc1ca_aff6_ac85_a4ad, 0xc227_e44a_b6a0_8dda),
            (0xc06e_1c94_3f41_c961, 0xc0ca_f514_3110_05b3),
            (0x4386_25a8_94c1_8421, 0x43e3_d3be_0492_fbda),
            (0xbc58_4c6b_2900_2f4e, 0xbcb5_c0ce_edd6_22ff),
            (0xbdd8_165e_4204_a614, 0xbe35_906b_7d6d_c2f4),
            (0xbd69_0e4c_a7e1_8d38, 0xbdc6_6e61_251f_bb26),
            (0x4323_9d39_ea19_7c3f, 0x4381_8f3c_4890_6dec),
            (0xbcda_68a9_cd87_706d, 0xbd37_a475_e4b9_0796),
            (0xbd03_30cb_5274_7098, 0xbd61_2e29_7fd0_5a45),
            (0x4195_b0f0_47be_2b84, 0x41f3_6b3f_c80f_a1fc),
            (0xbf38_78cd_d392_4613, 0xbf95_e88b_5120_6766),
            (0xc1fb_c1b1_d843_5add, 0xc258_d959_44c8_a19a),
            (0x3da0_8580_4c73_bb20, 0x3dfd_94e4_87c2_fef9),
            (0xbdf1_2a3f_ecb7_5197, 0xbe4e_bbdf_b2b8_957e),
            (0x3e36_8b67_c70c_32de, 0x3e94_2ed4_ae46_081e),
            (0x415a_ceca_f865_a989, 0x41b7_ffe4_43b8_c371),
            (0xbd9c_505c_342e_e0b1, 0xbdf9_5911_c8e9_8c3b),
            (0x410c_1a81_271b_5db8, 0x4169_28da_f98e_a941),
            (0xbf83_a378_8b13_f1a8, 0xbfe1_94d3_734d_592a),
            (0x40f0_753e_3f7d_4a28, 0x414d_77c8_6847_08eb),
            (0x4056_5f5d_d95f_f3c6, 0x40b4_0767_bc35_3d69),
            (0xc250_0535_6dc2_b6af, 0xc2ac_af2f_991b_a627),
            (0xc25b_7f4d_ecc2_e622, 0xc2b8_9de9_badc_39e2),
            (0x3eb3_43a4_4313_4338, 0x3f11_3f09_0297_06c8),
            (0xc082_6649_2672_a3e0, 0xc0e0_78dd_f935_ee3e),
            (0x3bf4_ab02_0375_3133, 0x3c52_80c1_ae21_9710),
            (0xc10f_bb88_ab86_217c, 0xc16c_6890_d240_afb6),
            (0xbd5f_1161_aa0f_ccc6, 0xbdbb_d03c_c581_1f43),
            (0x3fab_db8e_8f43_03cd, 0x4008_f080_70da_a9bd),
            (0xbcc7_e392_049e_f31c, 0xbd25_62f1_7d32_2790),
        ],
    ),
    (
        "arcsec_to_rad",
        &[
            (0x3ff0_0000_0000_0000, 0x3ed4_55a5_b2ff_8f9d),
            (0x3fe0_0000_0000_0000, 0x3ec4_55a5_b2ff_8f9d),
            (0x4056_8000_0000_0000, 0x3f3c_9871_03b7_61f5),
            (0x4066_8000_0000_0000, 0x3f4c_9871_03b7_61f5),
            (0x4076_8000_0000_0000, 0x3f5c_9871_03b7_61f5),
            (0x3f50_624d_d2f1_a9fc, 0x3e34_d295_322c_9b41),
            (0x412e_8480_0000_0000, 0x4013_647d_f9e8_c0d2),
            (0x40fe_240c_9fbe_76c9, 0x3fe3_2733_b7d9_b099),
            (0x3fb9_9999_9999_999a, 0x3ea0_4484_8f32_d94b),
            (0x4008_0000_0000_0000, 0x3eee_8078_8c7f_576c),
            (0xc1ca_aff6_ac85_a4ad, 0xc0b0_f567_c3a5_a195),
            (0xc06e_1c94_3f41_c961, 0xbf53_2274_7d1c_fe48),
            (0x4386_25a8_94c1_8421, 0x426c_25a0_25a7_59b3),
            (0xbc58_4c6b_2900_2f4e, 0xbb3e_e197_9071_671c),
            (0xbdd8_165e_4204_a614, 0xbcbe_9ce6_1b6a_266c),
            (0xbd69_0e4c_a7e1_8d38, 0xbc4f_d7ff_44db_9ba1),
            (0x4323_9d39_ea19_7c3f, 0x4208_ed86_c91f_2eff),
            (0xbcda_68a9_cd87_706d, 0xbbc0_c818_e2e3_3adf),
            (0xbd03_30cb_5274_7098, 0xbbe8_63b8_1d19_b583),
            (0x4195_b0f0_47be_2b84, 0x407b_9148_f983_a44b),
            (0xbf38_78cd_d392_4613, 0xbe1f_1a00_7d7c_394b),
            (0xc1fb_c1b1_d843_5add, 0xc0e1_a359_6190_a825),
            (0x3da0_8580_4c73_bb20, 0x3c84_ff50_b2b6_fc6e),
            (0xbdf1_2a3f_ecb7_5197, 0xbcd5_d0b2_1f3a_1f97),
            (0x3e36_8b67_c70c_32de, 0x3d1c_a6ef_c9f5_dfb9),
            (0x415a_ceca_f865_a989, 0x4041_08fe_f69e_4a03),
            (0xbd9c_505c_342e_e0b1, 0xbc81_fe01_b244_7658),
            (0x410c_1a81_271b_5db8, 0x3ff1_dbc8_a55e_cca8),
            (0xbf83_a378_8b13_f1a8, 0xbe68_f576_7fbb_2ce6),
            (0x40f0_753e_3f7d_4a28, 0x3fd4_eaa7_1b3f_09d4),
            (0x4056_5f5d_d95f_f3c6, 0x3f3c_6ef7_a40c_dcee),
            (0xc250_0535_6dc2_b6af, 0xc134_5c44_5dfb_4fe2),
            (0xc25b_7f4d_ecc2_e622, 0xc141_7929_3d30_3da7),
            (0x3eb3_43a4_4313_4338, 0x3d98_7bac_2d8e_9470),
            (0xc082_6649_2672_a3e0, 0xbf67_6259_6155_bb7d),
            (0x3bf4_ab02_0375_3133, 0x3ada_4465_09c2_3518),
            (0xc10f_bb88_ab86_217c, 0xbff4_2a23_deb4_baaf),
            (0xbd5f_1161_aa0f_ccc6, 0xbc43_be04_151d_fc08),
            (0x3fab_db8e_8f43_03cd, 0x3e91_b3c8_8c21_42af),
            (0xbcc7_e392_049e_f31c, 0xbbae_5c56_e392_55bd),
        ],
    ),
    (
        "grad_to_rad",
        &[
            (0x3ff0_0000_0000_0000, 0x3f90_15bf_9217_271a),
            (0x3fe0_0000_0000_0000, 0x3f80_15bf_9217_271a),
            (0x4056_8000_0000_0000, 0x3ff6_9e95_6570_8efc),
            (0x4066_8000_0000_0000, 0x4006_9e95_6570_8efc),
            (0x4076_8000_0000_0000, 0x4016_9e95_6570_8efc),
            (0x3f50_624d_d2f1_a9fc, 0x3ef0_7893_0430_48cf),
            (0x412e_8480_0000_0000, 0x40ce_adfb_4c5d_390c),
            (0x40fe_240c_9fbe_76c9, 0x409e_4d04_d1db_6463),
            (0x3fb9_9999_9999_999a, 0x3f59_bc65_b68b_71c3),
            (0x4008_0000_0000_0000, 0x3fa8_209f_5b22_baa7),
            (0xc1ca_aff6_ac85_a4ad, 0xc16a_d43d_2885_08a1),
            (0xc06e_1c94_3f41_c961, 0xc00e_4582_49ee_de47),
            (0x4386_25a8_94c1_8421, 0x4326_43c3_2dc8_e074),
            (0xbc58_4c6b_2900_2f4e, 0xbbf8_6d72_63c1_b410),
            (0xbdd8_165e_4204_a614, 0xbd78_371c_04af_7765),
            (0xbd69_0e4c_a7e1_8d38, 0xbd09_305b_6bf7_b69b),
            (0x4323_9d39_ea19_7c3f, 0x42c3_b7e3_1e17_2aad),
            (0xbcda_68a9_cd87_706d, 0xbc7a_8c8f_5ef1_7c23),
            (0xbd03_30cb_5274_7098, 0xbca3_4ae1_2304_d614),
            (0x4195_b0f0_47be_2b84, 0x4135_ce6c_395e_a176),
            (0xbf38_78cd_d392_4613, 0xbed8_9a11_6342_c351),
            (0xc1fb_c1b1_d843_5add, 0xc19b_e76c_6759_da02),
            (0x3da0_8580_4c73_bb20, 0x3d40_9bf5_555d_bead),
            (0xbdf1_2a3f_ecb7_5197, 0xbd91_4194_e5b3_79fd),
            (0x3e36_8b67_c70c_32de, 0x3dd6_aa0c_ad40_fd78),
            (0x415a_ceca_f865_a989, 0x40fa_f33b_5c28_6b17),
            (0xbd9c_505c_342e_e0b1, 0xbd3c_76d8_af06_4f3a),
            (0x410c_1a81_271b_5db8, 0x40ac_40b4_6d9e_f9c6),
            (0xbf83_a378_8b13_f1a8, 0xbf23_be2a_3c09_8f04),
            (0x40f0_753e_3f7d_4a28, 0x4090_8b9d_2f0d_5d46),
            (0x4056_5f5d_d95f_f3c6, 0x3ff6_7dc6_e344_2cc2),
            (0xc250_0535_6dc2_b6af, 0xc1f0_1afc_1457_4ab0),
            (0xc25b_7f4d_ecc2_e622, 0xc1fb_a4ae_3dcd_518a),
            (0x3eb3_43a4_4313_4338, 0x3e53_5dd3_b209_486a),
            (0xc082_6649_2672_a3e0, 0xc022_7f4b_b37e_50cf),
            (0x3bf4_ab02_0375_3133, 0x3b94_c719_ec38_1eff),
            (0xc10f_bb88_ab86_217c, 0xc0af_e6aa_bf53_eb57),
            (0xbd5f_1161_aa0f_ccc6, 0xbcff_3b9c_7568_6fb8),
            (0x3fab_db8e_8f43_03cd, 0x3f4c_016c_45b0_9e7e),
            (0xbcc7_e392_049e_f31c, 0xbc68_040a_bb03_40d2),
        ],
    ),
    (
        "rad_to_grad",
        &[
            (0x3ff0_0000_0000_0000, 0x404f_d4bb_ab8b_494c),
            (0x3fe0_0000_0000_0000, 0x403f_d4bb_ab8b_494c),
            (0x4056_8000_0000_0000, 0x40b6_6193_f49d_ef8a),
            (0x4066_8000_0000_0000, 0x40c6_6193_f49d_ef8a),
            (0x4076_8000_0000_0000, 0x40d6_6193_f49d_ef8a),
            (0x3f50_624d_d2f1_a9fc, 0x3fb0_4c26_be3b_06cf),
            (0x412e_8480_0000_0000, 0x418e_5b3c_c9e4_e173),
            (0x40fe_240c_9fbe_76c9, 0x415d_fb4b_d2a0_6b7d),
            (0x3fb9_9999_9999_999a, 0x4019_76fc_893c_3aa4),
            (0x4008_0000_0000_0000, 0x4067_df8c_c0a8_76f9),
            (0xc1ca_aff6_ac85_a4ad, 0xc22a_8be1_3cb2_64ba),
            (0xc06e_1c94_3f41_c961, 0xc0cd_f3dd_8bd8_e9e3),
            (0x4386_25a8_94c1_8421, 0x43e6_07b6_afbf_c280),
            (0xbc58_4c6b_2900_2f4e, 0xbcb8_2b90_967c_26e3),
            (0xbdd8_165e_4204_a614, 0xbe37_f5cc_c441_1181),
            (0xbd69_0e4c_a7e1_8d38, 0xbdc8_ec6b_f05c_2546),
            (0x4323_9d39_ea19_7c3f, 0x4383_82b4_c267_9695),
            (0xbcda_68a9_cd87_706d, 0xbd3a_44f4_c53f_5dc3),
            (0xbd03_30cb_5274_7098, 0xbd63_16d8_c6e7_80be),
            (0x4195_b0f0_47be_2b84, 0x41f5_939c_339f_978a),
            (0xbf38_78cd_d392_4613, 0xbf98_57b7_3db2_3a00),
            (0xc1fb_c1b1_d843_5add, 0xc25b_9c2a_4c6d_2556),
            (0x3da0_8580_4c73_bb20, 0x3e00_6f29_a0c1_aa19),
            (0xbdf1_2a3f_ecb7_5197, 0xbe51_130a_7fbb_e146),
            (0x3e36_8b67_c70c_32de, 0x3e96_6cec_4fdc_0905),
            (0x415a_ceca_f865_a989, 0x41ba_aa8b_d977_f59b),
            (0xbd9c_505c_342e_e0b1, 0xbdfc_2a13_c2ca_9bd0),
            (0x410c_1a81_271b_5db8, 0x416b_f481_8710_4a48),
            (0xbf83_a378_8b13_f1a8, 0xbfe3_88ea_f1e4_2a2f),
            (0x40f0_753e_3f7d_4a28, 0x4150_5efd_8f43_e882),
            (0x4056_5f5d_d95f_f3c6, 0x40b6_411d_ed90_7d20),
            (0xc250_0535_6dc2_b6af, 0xc2af_df18_713b_2a65),
            (0xc25b_7f4d_ecc2_e622, 0xc2bb_5a20_24f4_b218),
            (0x3eb3_43a4_4313_4338, 0x3f13_2998_3bc4_406c),
            (0xc082_6649_2672_a3e0, 0xc0e2_4d68_6a3b_ec45),
            (0x3bf4_ab02_0375_3133, 0x3c54_8f10_16cf_fd2f),
            (0xc10f_bb88_ab86_217c, 0xc16f_90a0_e99d_3503),
            (0xbd5f_1161_aa0f_ccc6, 0xbdbe_e75f_f7e4_cd67),
            (0x3fab_db8e_8f43_03cd, 0x400b_b5e4_0b9d_a027),
            (0xbcc7_e392_049e_f31c, 0xbd27_c345_35c5_f312),
        ],
    ),
    (
        "eV_to_J",
        &[
            (0x3ff0_0000_0000_0000, 0x3c07_a4da_290c_1653),
            (0x3fe0_0000_0000_0000, 0x3bf7_a4da_290c_1653),
            (0x4056_8000_0000_0000, 0x3c70_9fe9_64dc_7fb3),
            (0x4066_8000_0000_0000, 0x3c80_9fe9_64dc_7fb3),
            (0x4076_8000_0000_0000, 0x3c90_9fe9_64dc_7fb3),
            (0x3f50_624d_d2f1_a9fc, 0x3b68_361e_e25a_3389),
            (0x412e_8480_0000_0000, 0x3d46_8c73_1dcd_4afb),
            (0x40fe_240c_9fbe_76c9, 0x3d16_452f_4fc3_287c),
            (0x3fb9_9999_9999_999a, 0x3bd2_ea48_20d6_7843),
            (0x4008_0000_0000_0000, 0x3c21_bba3_9ec9_10bf),
            (0xc1ca_aff6_ac85_a4ad, 0xbde3_b7f5_0d30_3b3f),
            (0xc06e_1c94_3f41_c961, 0xbc86_3faa_4e8d_6681),
            (0x4386_25a8_94c1_8421, 0x3fa0_5d29_27da_89b5),
            (0xbc58_4c6b_2900_2f4e, 0xb871_f41a_526f_612c),
            (0xbdd8_165e_4204_a614, 0xb9f1_cc2a_99cd_92e7),
            (0xbd69_0e4c_a7e1_8d38, 0xb982_835b_3314_8234),
            (0x4323_9d39_ea19_7c3f, 0x3f3c_fc1a_42aa_f321),
            (0xbcda_68a9_cd87_706d, 0xb8f3_8346_7cdb_8a55),
            (0xbd03_30cb_5274_7098, 0xb91c_5bde_14a2_5732),
            (0x4195_b0f0_47be_2b84, 0x3db0_06eb_6438_2bfe),
            (0xbf38_78cd_d392_4613, 0xbb52_14e5_e54b_a770),
            (0xc1fb_c1b1_d843_5add, 0xbe14_8235_be0a_fba9),
            (0x3da0_8580_4c73_bb20, 0x39b8_6a22_164c_a4d7),
            (0xbdf1_2a3f_ecb7_5197, 0xba09_5d96_ff32_1101),
            (0x3e36_8b67_c70c_32de, 0x3a50_a856_bd8b_0010),
            (0x415a_ceca_f865_a989, 0x3d73_cebc_75eb_eb62),
            (0xbd9c_505c_342e_e0b1, 0xb9b4_eb9f_25d2_ed45),
            (0x410c_1a81_271b_5db8, 0x3d24_c3d4_42a0_850d),
            (0xbf83_a378_8b13_f1a8, 0xbb9d_0554_a12a_59a4),
            (0x40f0_753e_3f7d_4a28, 0x3d08_521b_a0f7_de45),
            (0x4056_5f5d_d95f_f3c6, 0x3c70_87cc_bb98_ff73),
            (0xc250_0535_6dc2_b6af, 0xbe67_ac8c_a17e_5d15),
            (0xc25b_7f4d_ecc2_e622, 0xbe74_5127_e811_d7f7),
            (0x3eb3_43a4_4313_4338, 0x3acc_77b8_1f25_2a80),
            (0xc082_6649_2672_a3e0, 0xbc9b_309c_75e7_9052),
            (0x3bf4_ab02_0375_3133, 0x380e_8ac5_889d_2950),
            (0xc10f_bb88_ab86_217c, 0xbd27_7243_adfe_cb91),
            (0xbd5f_1161_aa0f_ccc6, 0xb976_f48b_1506_4407),
            (0x3fab_db8e_8f43_03cd, 0x3bc4_9551_9d0d_6694),
            (0xbcc7_e392_049e_f31c, 0xb8e1_a6a2_1ca1_45ec),
        ],
    ),
    (
        "J_to_eV",
        &[
            (0x3ff0_0000_0000_0000, 0x43d5_a792_dbcb_6718),
            (0x3fe0_0000_0000_0000, 0x43c5_a792_dbcb_6718),
            (0x4056_8000_0000_0000, 0x443e_73a6_8516_08fa),
            (0x4066_8000_0000_0000, 0x444e_73a6_8516_08fa),
            (0x4076_8000_0000_0000, 0x445e_73a6_8516_08fa),
            (0x3f50_624d_d2f1_a9fc, 0x4336_2c9e_933e_e05b),
            (0x412e_8480_0000_0000, 0x4514_a6c3_8a24_c6de),
            (0x40fe_240c_9fbe_76c9, 0x44e4_657e_be36_8cf5),
            (0x3fb9_9999_9999_999a, 0x43a1_52db_e309_1f47),
            (0x4008_0000_0000_0000, 0x43f0_3dae_24d8_8d52),
            (0xc1ca_aff6_ac85_a4ad, 0xc5b2_0f3a_aaaf_6313),
            (0xc06e_1c94_3f41_c961, 0xc454_6070_a064_a8f7),
            (0x4386_25a8_94c1_8421, 0x476d_f961_9aad_ee98),
            (0xbc58_4c6b_2900_2f4e, 0xc040_7164_a5b4_7384),
            (0xbdd8_165e_4204_a614, 0xc1c0_4cd1_22bb_d309),
            (0xbd69_0e4c_a7e1_8d38, 0xc150_f497_eb8f_03c7),
            (0x4323_9d39_ea19_7c3f, 0x470a_8bc9_17de_b721),
            (0xbcda_68a9_cd87_706d, 0xc0c1_defa_d7f4_5d75),
            (0xbd03_30cb_5274_7098, 0xc0e9_f908_4b44_9939),
            (0x4195_b0f0_47be_2b84, 0x457d_5b69_42a0_922e),
            (0xbf38_78cd_d392_4613, 0xc320_8f6d_d441_8477),
            (0xc1fb_c1b1_d843_5add, 0xc5e2_c876_f06e_4523),
            (0x3da0_8580_4c73_bb20, 0x4186_5c41_749d_eb82),
            (0xbdf1_2a3f_ecb7_5197, 0xc1d7_3b3a_6f3b_2620),
            (0x3e36_8b67_c70c_32de, 0x421e_8316_2e33_40d3),
            (0x415a_ceca_f865_a989, 0x4542_2417_6de2_ba68),
            (0xbd9c_505c_342e_e0b1, 0xc183_2901_d48d_b635),
            (0x410c_1a81_271b_5db8, 0x44f3_0490_0da3_5550),
            (0xbf83_a378_8b13_f1a8, 0xc36a_943c_b228_ecf0),
            (0x40f0_753e_3f7d_4a28, 0x44d6_4640_7cf9_1df8),
            (0x4056_5f5d_d95f_f3c6, 0x443e_477b_ea68_e7ae),
            (0xc250_0535_6dc2_b6af, 0xc635_ae9f_8953_ae47),
            (0xc25b_7f4d_ecc2_e622, 0xc642_9b89_b3bd_0dda),
            (0x3eb3_43a4_4313_4338, 0x429a_128a_6c63_0402),
            (0xc082_6649_2672_a3e0, 0xc468_e6f4_8015_3408),
            (0x3bf4_ab02_0375_3133, 0x3fdb_f8e9_3deb_b1b7),
            (0xc10f_bb88_ab86_217c, 0xc4f5_793e_0363_dbc9),
            (0xbd5f_1161_aa0f_ccc6, 0xc145_0619_61fb_e77e),
            (0x3fab_db8e_8f43_03cd, 0x4392_d9f7_3711_b811),
            (0xbcc7_e392_049e_f31c, 0xc0b0_2a71_176f_ea11),
        ],
    ),
    (
        "Da_to_kg",
        &[
            (0x3ff0_0000_0000_0000, 0x3a60_71f7_78ed_6aaf),
            (0x3fe0_0000_0000_0000, 0x3a50_71f7_78ed_6aaf),
            (0x4056_8000_0000_0000, 0x3ac7_2044_020d_de06),
            (0x4066_8000_0000_0000, 0x3ad7_2044_020d_de06),
            (0x4076_8000_0000_0000, 0x3ae7_2044_020d_de06),
            (0x3f50_624d_d2f1_a9fc, 0x39c0_d701_81f9_423d),
            (0x412e_8480_0000_0000, 0x3b9f_5ddf_cc1e_9624),
            (0x40fe_240c_9fbe_76c9, 0x3b6e_fabd_6937_279e),
            (0x3fb9_9999_9999_999a, 0x3a2a_4ff2_5b15_777f),
            (0x4008_0000_0000_0000, 0x3a78_aaf3_3564_2007),
            (0xc1ca_aff6_ac85_a4ad, 0xbc3b_6e0e_30cb_d64d),
            (0xc06e_1c94_3f41_c961, 0xbade_f30f_d306_427c),
            (0x4386_25a8_94c1_8421, 0x3df6_c369_1819_a51a),
            (0xbc58_4c6b_2900_2f4e, 0xb6c8_f97e_b12d_bb3a),
            (0xbdd8_165e_4204_a614, 0xb848_c1f0_cb13_30f0),
            (0xbd69_0e4c_a7e1_8d38, 0xb7d9_c0c5_2f61_9fa3),
            (0x4323_9d39_ea19_7c3f, 0x3d94_28ef_b28b_1c34),
            (0xbcda_68a9_cd87_706d, 0xb74b_24c5_7419_29c1),
            (0xbd03_30cb_5274_7098, 0xb773_b97c_c0b8_477c),
            (0x4195_b0f0_47be_2b84, 0x3c06_4b71_6827_15c2),
            (0xbf38_78cd_d392_4613, 0xb9a9_271d_8317_198c),
            (0xc1fb_c1b1_d843_5add, 0xbc6c_8767_203c_c522),
            (0x3da0_8580_4c73_bb20, 0x3810_fb2e_b05a_b297),
            (0xbdf1_2a3f_ecb7_5197, 0xb861_a483_ce25_8a4b),
            (0x3e36_8b67_c70c_32de, 0x38a7_2bfd_0670_1b6a),
            (0x415a_ceca_f865_a989, 0x3bcb_8dbe_14da_0e4d),
            (0xbd9c_505c_342e_e0b1, 0xb80d_1a09_aded_a7ff),
            (0x410c_1a81_271b_5db8, 0x3b7c_e2af_04d1_2756),
            (0xbf83_a378_8b13_f1a8, 0xb9f4_2f5a_ce6c_81f5),
            (0x40f0_753e_3f7d_4a28, 0x3b60_ea78_d572_2c65),
            (0x4056_5f5d_d95f_f3c6, 0x3ac6_feb9_697f_2ed2),
            (0xc250_0535_6dc2_b6af, 0xbcc0_7752_0097_7cf8),
            (0xc25b_7f4d_ecc2_e622, 0xbccc_432a_5030_cb16),
            (0x3eb3_43a4_4313_4338, 0x3923_ccdb_f0fd_f117),
            (0xc082_6649_2672_a3e0, 0xbaf2_e958_212b_997a),
            (0x3bf4_ab02_0375_3133, 0x3665_3e39_6dd1_f036),
            (0xc10f_bb88_ab86_217c, 0xbb80_4ec7_f7d2_5d92),
            (0xbd5f_1161_aa0f_ccc6, 0xb7cf_eeac_f2f3_eac9),
            (0x3fab_db8e_8f43_03cd, 0x3a1c_a1fc_0e0b_9bd4),
            (0xbcc7_e392_049e_f31c, 0xb738_8dba_b98a_fe55),
        ],
    ),
    (
        "kg_to_Da",
        &[
            (0x3ff0_0000_0000_0000, 0x457f_223c_a7c7_ec49),
            (0x3fe0_0000_0000_0000, 0x456f_223c_a7c7_ec49),
            (0x4056_8000_0000_0000, 0x45e5_e412_a5f8_9223),
            (0x4066_8000_0000_0000, 0x45f5_e412_a5f8_9223),
            (0x4076_8000_0000_0000, 0x4605_e412_a5f8_9223),
            (0x3f50_624d_d2f1_a9fc, 0x44df_e185_ca87_16d2),
            (0x412e_8480_0000_0000, 0x46bd_b102_a072_2552),
            (0x40fe_240c_9fbe_76c9, 0x468d_532b_a9ca_5994),
            (0x3fb9_9999_9999_999a, 0x4548_e830_8639_89d4),
            (0x4008_0000_0000_0000, 0x4597_59ad_7dd5_f136),
            (0xc1ca_aff6_ac85_a4ad, 0xc759_f704_8314_53aa),
            (0xc06e_1c94_3f41_c961, 0xc5fd_4be7_0e42_2d52),
            (0x4386_25a8_94c1_8421, 0x4915_8c2d_4df4_8208),
            (0xbc58_4c6b_2900_2f4e, 0xc1e7_a407_1043_48c9),
            (0xbdd8_165e_4204_a614, 0xc367_6f70_bc56_9a7b),
            (0xbd69_0e4c_a7e1_8d38, 0xc2f8_60a8_f243_c536),
            (0x4323_9d39_ea19_7c3f, 0x48b3_154c_55f5_1333),
            (0xbcda_68a9_cd87_706d, 0xc269_b1a5_c23b_2a37),
            (0xbd03_30cb_5274_7098, 0xc292_abcd_3004_2d9f),
            (0x4195_b0f0_47be_2b84, 0x4725_1a9d_e249_e531),
            (0xbf38_78cd_d392_4613, 0xc4c7_cf36_2277_c21a),
            (0xc1fb_c1b1_d843_5add, 0xc78b_0156_b32c_beb3),
            (0x3da0_8580_4c73_bb20, 0x4330_1301_7351_d55a),
            (0xbdf1_2a3f_ecb7_5197, 0xc380_b34b_5a74_b0c0),
            (0x3e36_8b67_c70c_32de, 0x43c5_ef2b_62ae_426f),
            (0x415a_ceca_f865_a989, 0x46ea_1503_2895_a930),
            (0xbd9c_505c_342e_e0b1, 0xc32b_8c24_5fa6_0351),
            (0x410c_1a81_271b_5db8, 0x469b_57be_8c28_1f8c),
            (0xbf83_a378_8b13_f1a8, 0xc513_1b5f_b048_eabf),
            (0x40f0_753e_3f7d_4a28, 0x4680_0330_11c4_3b7d),
            (0x4056_5f5d_d95f_f3c6, 0x45e5_c452_a669_59b7),
            (0xc250_0535_6dc2_b6af, 0xc7df_2c5f_51b9_6659),
            (0xc25b_7f4d_ecc2_e622, 0xc7ea_c0be_df06_a354),
            (0x3eb3_43a4_4313_4338, 0x4442_be23_8357_477c),
            (0xc082_6649_2672_a3e0, 0xc611_e6c6_6b39_d25a),
            (0x3bf4_ab02_0375_3133, 0x4184_1bc6_d27e_8d3c),
            (0xc10f_bb88_ab86_217c, 0xc69e_df9f_cd61_6257),
            (0xbd5f_1161_aa0f_ccc6, 0xc2ee_3a13_f7ff_ed35),
            (0x3fab_db8e_8f43_03cd, 0x453b_1a80_2ffc_4a8d),
            (0xbcc7_e392_049e_f31c, 0xc257_3e04_8791_8eb0),
        ],
    ),
];

#[test]
fn factors_with_pi_or_a_power_of_ten_are_correctly_rounded() {
    let reg = registry().unwrap();
    let mut rows = 0_u32;
    for (name, table) in ORACLE {
        let (from, to) = name.split_once("_to_").unwrap();
        let conv = reg.conversion(from, to).unwrap();
        if name.contains("rad") || name.contains("grad") {
            assert_ne!(conv.factor().pi_exp(), 0, "{name} must exercise pi");
        }
        for &(x_bits, want_bits) in *table {
            let x = f64::from_bits(x_bits);
            let got = conv.apply_exact_f64(x).unwrap();
            assert_eq!(
                got.to_bits(),
                want_bits,
                "{name}: apply_exact_f64({x:e}) = {got:e}, oracle {:e}",
                f64::from_bits(want_bits)
            );
            // the fast path stays within the documented 1 ulp
            let fast = conv.apply(x);
            let (a, b) = (fast.to_bits(), got.to_bits());
            assert_eq!(fast.is_sign_negative(), got.is_sign_negative());
            assert!(
                a.abs_diff(b) <= 1,
                "{name}: fast {fast:e} vs exact {got:e} at {x:e}"
            );
            rows += 1;
        }
    }
    assert!(rows >= 300, "{rows}");
}
