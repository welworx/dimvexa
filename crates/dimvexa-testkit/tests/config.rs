// SPDX-License-Identifier: MIT OR Apache-2.0
//! User configs (ADR-009) applied to the shipped table: what a mapping may and may not do.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dimvexa::{Ratio, Registry, registry};
use dimvexa_config::{Config, apply};

fn cfg(text: &str, origin: &str) -> Config {
    Config::parse(text, origin).unwrap()
}

fn with(text: &str) -> Registry {
    let mut reg = registry().unwrap();
    apply(&mut reg, &[cfg(text, "test.toml")]).unwrap();
    reg
}

/// Applying `text` to the shipped table must be refused; returns the error text.
fn refused(text: &str) -> String {
    let mut reg = registry().unwrap();
    let err = apply(&mut reg, &[cfg(text, "test.toml")]).unwrap_err();
    assert_eq!(err.code(), "invalid_definition", "{text}");
    err.to_string()
}

fn factor(reg: &Registry, from: &str, to: &str) -> Ratio {
    reg.conversion(from, to).unwrap().factor()
}

#[test]
fn a_mapping_resolves_an_ambiguous_name() {
    let base = registry().unwrap();
    for name in ["mt", "ton"] {
        assert_eq!(
            base.conversion(name, "kg").unwrap_err().code(),
            "ambiguous_symbol"
        );
    }

    let reg = with("[map]\nmt = \"tonne\"\nton = \"ton_short\"\n");
    assert_eq!(factor(&reg, "mt", "kg"), Ratio::from_int(1000));
    assert_eq!(
        factor(&reg, "ton", "kg"),
        Ratio::new(45_359_237, 50_000).unwrap()
    );
    // The mapping behaves like an alias: prefixes and compound expressions work.
    assert_eq!(factor(&reg, "kmt", "t"), Ratio::from_int(1000));
    assert_eq!(factor(&reg, "mt/h", "kg/h"), Ratio::from_int(1000));
    // Explicit names are unaffected.
    assert_eq!(
        factor(&reg, "ton_long", "kg"),
        factor(&base, "ton_long", "kg")
    );
    // Other ambiguous symbols stay refused.
    assert_eq!(
        reg.conversion("gal", "L").unwrap_err().code(),
        "ambiguous_symbol"
    );
}

#[test]
fn a_mapping_can_add_a_new_name() {
    let reg = with("[map]\nmetric_ton = \"tonne\"\nUSton = \"ton_short\"\n");
    assert_eq!(factor(&reg, "metric_ton", "kg"), Ratio::from_int(1000));
    assert_eq!(
        factor(&reg, "USton", "ton_short"),
        Ratio::ONE,
        "an alias means exactly what its target means"
    );
}

#[test]
fn existing_symbols_cannot_be_redefined() {
    for name in ["m", "t", "kg", "psi", "km", "kPa", "degC", "ton_short"] {
        let msg = refused(&format!("[map]\n{name} = \"tonne\"\n"));
        assert!(msg.contains("already means a unit"), "{name}: {msg}");
        assert!(msg.contains("test.toml:2"), "{msg}");
    }
}

#[test]
fn a_target_must_be_a_unit_of_the_table() {
    let msg = refused("[map]\nfoo = \"nope\"\n");
    assert!(msg.contains("not a unit symbol or alias"), "{msg}");
    // No factors, expressions or prefixed forms as targets: a mapping only points at a unit.
    for target in ["km/h", "ktonne", "1000 kg"] {
        refused(&format!("[map]\nfoo = \"{target}\"\n"));
    }
    refused("[map]\n\"two words\" = \"tonne\"\n");
}

#[test]
fn a_later_config_overrides_an_earlier_mapping() {
    let (first, second) = (
        cfg("[map]\nton = \"ton_short\"\n", "first.toml"),
        cfg("[map]\nton = \"ton_long\"\n", "second.toml"),
    );
    let mut reg = registry().unwrap();
    let applied = apply(&mut reg, &[first.clone(), second.clone()]).unwrap();
    assert_eq!(factor(&reg, "ton", "kg"), factor(&reg, "ton_long", "kg"));
    assert_eq!(applied.mappings.len(), 1);
    assert_eq!(applied.mappings[0].origin, "second.toml");

    let mut reg = registry().unwrap();
    apply(&mut reg, &[second, first]).unwrap();
    assert_eq!(factor(&reg, "ton", "kg"), factor(&reg, "ton_short", "kg"));
}

#[test]
fn a_refused_config_leaves_the_registry_untouched() {
    let mut reg = registry().unwrap();
    let bad = cfg("[map]\nmt = \"tonne\"\nm = \"tonne\"\n", "mixed.toml");
    let err = apply(&mut reg, &[bad]).unwrap_err();
    assert!(err.to_string().contains("mixed.toml:3"), "{err}");
    assert_eq!(
        reg.conversion("mt", "kg").unwrap_err().code(),
        "ambiguous_symbol"
    );
}

#[test]
fn applying_twice_is_refused_so_overrides_stay_explicit() {
    // Overrides are resolved inside one call; a second call finds the name taken.
    let mut reg = registry().unwrap();
    apply(&mut reg, &[cfg("[map]\nton = \"ton_short\"\n", "a.toml")]).unwrap();
    let err = apply(&mut reg, &[cfg("[map]\nton = \"ton_long\"\n", "b.toml")]).unwrap_err();
    assert!(err.to_string().contains("already means a unit"), "{err}");
}

const WATER: &str = "[[constant]]\nname = \"water_20C\"\n\
value = \"9982067/10000\"\nunit = \"kg/m3\"\nexactness = \"measured\"\nconditions = \"air-free water, 20 degC, 101.325 kPa\"\n\
source = \"Tanaka et al., Metrologia 38 (2001) 301\"\n";

fn water() -> dimvexa_config::Density {
    let reg = registry().unwrap();
    cfg(WATER, "water.toml").constants()[0]
        .to_density(&reg)
        .unwrap()
}

#[test]
fn a_density_turns_volume_into_mass_and_back_exactly() {
    let (reg, b) = (registry().unwrap(), water());
    let density = Ratio::new(9_982_067, 10_000).unwrap(); // kg/m3
    let (litre, kg) = (reg.parse("L").unwrap(), reg.parse("kg").unwrap());
    // 1 L = 0.001 m3, so 1 L of this water is 0.9982067 kg, exactly.
    let c = b.conversion(&litre, &kg).unwrap();
    assert_eq!(c.factor(), Ratio::new(9_982_067, 10_000_000).unwrap());
    // And back: the two directions are exact inverses.
    let back = b.conversion(&kg, &litre).unwrap();
    assert_eq!(c.factor().checked_mul(back.factor()).unwrap(), Ratio::ONE);
    // Other units of the same dimensions, and flows (per time).
    let m3 = reg.parse("m3").unwrap();
    assert_eq!(b.conversion(&m3, &kg).unwrap().factor(), density);
    let gal = reg.parse("gal_us").unwrap();
    let t = reg.parse("tonne").unwrap();
    let want = gal
        .factor()
        .checked_mul(density)
        .unwrap()
        .checked_div(t.factor())
        .unwrap();
    assert_eq!(b.conversion(&gal, &t).unwrap().factor(), want);
    let (lph, kgph) = (reg.parse("L/h").unwrap(), reg.parse("kg/h").unwrap());
    assert_eq!(
        b.conversion(&lph, &kgph).unwrap().factor(),
        c.factor(),
        "a per-time flow converts with the same factor"
    );
}

#[test]
fn a_density_does_nothing_for_anything_but_a_density() {
    let (reg, b) = (registry().unwrap(), water());
    // Not mass <-> volume: refused with the ordinary mismatch error, not guessed.
    for (from, to) in [
        ("L", "m"),
        ("kg", "s"),
        ("L", "L2"),
        ("kg", "kg"),
        ("m3/h", "kg"),
    ] {
        let (f, t) = (reg.parse(from).unwrap(), reg.parse(to).unwrap());
        let err = b.conversion(&f, &t).unwrap_err();
        assert_eq!(err.code(), "dimension_mismatch", "{from} -> {to}");
    }
    // A temperature is neither a mass nor a volume, so a density cannot touch it.
    let (degc, kg) = (reg.parse("degC").unwrap(), reg.parse("kg").unwrap());
    assert!(b.conversion(&degc, &kg).is_err());
    assert!(dimvexa_config::Density::density_needed(
        reg.parse("L").unwrap().dim(),
        kg.dim()
    ));
    assert!(!dimvexa_config::Density::density_needed(kg.dim(), kg.dim()));
}

const BARREL: &str = "[[unit]]\nsymbol = \"bbl_oil\"\nbase = \"gal_us\"\nfactor = \"42\"\n\
exactness = \"defined\"\nprefixes = \"si\"\nsource = \"NIST SP 811 (2008), Appendix B\"\n";

fn unit_text(symbol: &str, base: &str, factor: &str, extra: &str) -> String {
    format!(
        "[[unit]]\nsymbol = \"{symbol}\"\nbase = \"{base}\"\nfactor = \"{factor}\"\n\
         exactness = \"defined\"\nsource = \"test\"\n{extra}"
    )
}

#[test]
fn a_user_unit_is_an_exact_multiple_of_its_base() {
    let reg = {
        let mut reg = registry().unwrap();
        apply(&mut reg, &[cfg(BARREL, "oil.toml")]).unwrap();
        reg
    };
    // 42 US gallons = 42 * 3.785411784 L, exactly; the dimension (volume) comes from the base.
    let gal_l = factor(&reg, "gal_us", "L");
    assert_eq!(
        factor(&reg, "bbl_oil", "L"),
        gal_l.checked_mul(Ratio::from_int(42)).unwrap()
    );
    assert_eq!(
        factor(&reg, "bbl_oil", "m3"),
        Ratio::new(9_936_705_933, 62_500_000_000).unwrap()
    );
    assert_eq!(
        reg.conversion("bbl_oil", "kg").unwrap_err().code(),
        "dimension_mismatch"
    );
    // Its prefixes work as asked; compounds too.
    assert_eq!(
        factor(&reg, "Mbbl_oil", "bbl_oil"),
        Ratio::from_int(1_000_000)
    );
    assert_eq!(
        factor(&reg, "bbl_oil/d", "L/d"),
        factor(&reg, "bbl_oil", "L")
    );
    // Provenance: the user's source, namespace "user", and a note saying what it is.
    let unit = reg.get("bbl_oil").unwrap();
    assert_eq!(unit.namespace(), "user");
    assert_eq!(unit.source(), "NIST SP 811 (2008), Appendix B");
    assert!(
        unit.note().contains("1 bbl_oil = 42 gal_us"),
        "{}",
        unit.note()
    );
}

#[test]
fn a_user_unit_without_prefixes_takes_none() {
    let mut reg = registry().unwrap();
    apply(
        &mut reg,
        &[cfg(&unit_text("pallet", "kg", "800", ""), "p.toml")],
    )
    .unwrap();
    assert_eq!(factor(&reg, "pallet", "kg"), Ratio::from_int(800));
    assert_eq!(
        reg.conversion("kpallet", "kg").unwrap_err().code(),
        "unknown_unit"
    );
}

#[test]
fn a_user_unit_can_build_on_an_earlier_one_and_on_a_measured_base() {
    let mut reg = registry().unwrap();
    let text = format!(
        "{}{}{}",
        unit_text("pallet", "kg", "800", ""),
        unit_text("lorry", "pallet", "24", ""),
        // lb is defined; mmHg derives from measured data, so a unit on it is measured.
        unit_text("wmm", "mmHg", "2", "")
    );
    apply(&mut reg, &[cfg(&text, "p.toml")]).unwrap();
    assert_eq!(factor(&reg, "lorry", "kg"), Ratio::from_int(19_200));
    let exactness = |sym: &str| reg.get(sym).unwrap().exactness().name();
    assert_eq!(exactness("pallet"), "defined");
    assert_eq!(
        exactness("wmm"),
        reg.get("mmHg").unwrap().exactness().name()
    );
}

#[test]
fn a_user_unit_can_never_take_an_existing_name() {
    for name in ["m", "kg", "psi", "mm", "kPa", "t", "degC", "ton_short"] {
        let msg = refused(&unit_text(name, "kg", "2", ""));
        assert!(msg.contains("already means a unit"), "{name}: {msg}");
        assert!(msg.contains("test.toml:1"), "{msg}");
    }
    // A declared-ambiguous bare name is resolved with [map], not by defining a unit.
    let msg = refused(&unit_text("ton", "kg", "907", ""));
    assert!(msg.contains("ambiguous"), "{msg}");
}

#[test]
fn a_user_unit_needs_a_linear_base_that_exists() {
    for (base, needle) in [
        ("nope", "base 'nope'"),
        ("degC", "temperature point or has an offset"),
        ("K", "temperature point or has an offset"),
        ("kg +", "base 'kg +'"),
    ] {
        let msg = refused(&unit_text("thing", base, "2", ""));
        assert!(msg.contains(needle), "{base}: {msg}");
    }
    // Forward references are refused: a unit can only build on one defined before it.
    let text = format!(
        "{}{}",
        unit_text("lorry", "pallet", "24", ""),
        unit_text("pallet", "kg", "800", "")
    );
    let mut reg = registry().unwrap();
    assert!(apply(&mut reg, &[cfg(&text, "t.toml")]).is_err());
}

#[test]
fn a_mapping_can_point_at_a_user_unit_and_a_later_config_overrides_a_unit() {
    let mut reg = registry().unwrap();
    let first = cfg(
        &format!(
            "{}[map]\nbarrel = \"bbl_oil\"\n",
            unit_text("bbl_oil", "gal_us", "42", "")
        ),
        "a.toml",
    );
    let second = cfg(&unit_text("bbl_oil", "gal_us", "55", ""), "b.toml");
    let applied = apply(&mut reg, &[first, second]).unwrap();
    assert_eq!(applied.units.len(), 1);
    assert_eq!(applied.units[0].origin, "b.toml");
    assert_eq!(factor(&reg, "barrel", "gal_us"), Ratio::from_int(55));
}

#[test]
fn a_refused_unit_leaves_the_registry_untouched() {
    let mut reg = registry().unwrap();
    let len = reg.len();
    let text = format!(
        "{}{}",
        unit_text("pallet", "kg", "800", ""),
        unit_text("m", "kg", "2", "")
    );
    assert!(apply(&mut reg, &[cfg(&text, "t.toml")]).is_err());
    assert_eq!(reg.len(), len);
    assert_eq!(
        reg.conversion("pallet", "kg").unwrap_err().code(),
        "unknown_unit"
    );
}

#[test]
fn a_density_only_turns_mass_into_volume_not_other_dimensions_that_differ_by_a_density() {
    let (reg, b) = (registry().unwrap(), water());
    let dim = |s: &str| reg.parse(s).unwrap().dim();
    // These differ by exactly kg/m3 but are not mass <-> volume.
    for (from, to) in [
        ("kg/L", "m"),
        ("kg/m2", "mm"),
        ("m3/kg", "m3"),
        ("kg/m3", "kg/m3"),
        ("kg/s", "m3"),
        ("kg", "m3/s"),
    ] {
        assert!(
            !dimvexa_config::Density::density_needed(dim(from), dim(to)),
            "{from} -> {to}"
        );
        let (f, t) = (reg.parse(from).unwrap(), reg.parse(to).unwrap());
        assert!(b.conversion(&f, &t).is_err(), "{from} -> {to}");
    }
    // Mass <-> volume with the same power of time still works.
    for (from, to) in [("kg", "L"), ("L", "kg"), ("kg/h", "m3/h"), ("m3/s", "kg/s")] {
        assert!(
            dimvexa_config::Density::density_needed(dim(from), dim(to)),
            "{from} -> {to}"
        );
    }
    // A different power of time on the two sides does not.
    assert!(!dimvexa_config::Density::density_needed(
        dim("kg/h"),
        dim("m3")
    ));
}

#[test]
fn a_prefix_on_an_ambiguous_name_cannot_be_redefined() {
    // `cal` and `gal` are ambiguous, so `kcal`, `kgal` and `Mcal` are refused too. Defining
    // them would silently drop the prefix (1000 kcal would then be 1000 cal).
    let reg = registry().unwrap();
    for (name, target) in [("kcal", "cal_th"), ("kgal", "gal_us"), ("Mcal", "cal_IT")] {
        let msg = refused(&format!("[map]\n{name} = \"{target}\"\n"));
        assert!(msg.contains("already means a unit"), "{name}: {msg}");
        assert!(
            msg.contains(&format!("'{name}'")),
            "the name is in the error: {msg}"
        );
        let msg = refused(&unit_text(name, "kg", "2", ""));
        assert!(msg.contains("already means a unit"), "{name}: {msg}");
        assert_eq!(
            reg.conversion(name, "kg").unwrap_err().code(),
            "ambiguous_symbol",
            "{name} stays ambiguous"
        );
    }
    // The ambiguous bare name itself can still be resolved (that is what [map] is for).
    with("[map]\ncal = \"cal_th\"\n");
    // And `name_is_free` says why.
    assert!(reg.name_is_free("cal") && reg.name_is_free("pallet"));
    assert!(!reg.name_is_free("kcal") && !reg.name_is_free("km") && !reg.name_is_free("m"));
}

#[test]
fn overriding_a_unit_keeps_the_units_that_build_on_it_working() {
    let base = format!(
        "{}{}",
        unit_text("uu", "kg", "2", ""),
        unit_text("ub", "uu", "3", "")
    );
    let mut reg = registry().unwrap();
    let applied = apply(
        &mut reg,
        &[
            cfg(&base, "A.toml"),
            cfg(&unit_text("uu", "kg", "5", ""), "B.toml"),
        ],
    )
    .unwrap();
    assert_eq!(factor(&reg, "uu", "kg"), Ratio::from_int(5));
    assert_eq!(
        factor(&reg, "ub", "kg"),
        Ratio::from_int(15),
        "ub follows the new uu"
    );
    let order: Vec<&str> = applied.units.iter().map(|u| u.symbol.as_str()).collect();
    assert_eq!(order, ["uu", "ub"], "an override keeps its position");
    assert_eq!(applied.units[0].origin, "B.toml");
}

#[test]
fn a_user_unit_keeps_the_citation_of_its_base() {
    let mut reg = registry().unwrap();
    apply(&mut reg, &[cfg(BARREL, "oil.toml")]).unwrap();
    let note = reg.get("bbl_oil").unwrap().note().to_string();
    let base_source = reg.get("gal_us").unwrap().source().to_string();
    assert!(
        note.contains("(base gal_us:") && note.contains(&base_source),
        "{note}"
    );
}

#[test]
fn problems_name_the_offender_and_the_chain_rules() {
    // The name is in the message.
    let msg = refused("[map]\nMT = \"tonne\"\n");
    assert!(msg.contains("'MT'") && msg.contains("test.toml:2"), "{msg}");
    // A chain works only through earlier names; the error says what a target may be.
    let msg = refused("[map]\nton = \"mt\"\nmt = \"tonne\"\n");
    assert!(msg.contains("known at this point"), "{msg}");
    let mut reg = registry().unwrap();
    apply(
        &mut reg,
        &[cfg("[map]\nmt = \"tonne\"\nton = \"mt\"\n", "ok.toml")],
    )
    .unwrap();
    assert_eq!(factor(&reg, "ton", "kg"), Ratio::from_int(1000));
    // A base cannot be a mapped name; the error says so.
    let text = format!("[map]\nmt = \"tonne\"\n{}", unit_text("lot", "mt", "2", ""));
    let msg = refused(&text);
    assert!(msg.contains("mapped names do not count"), "{msg}");
}

#[test]
fn a_new_name_cannot_hide_a_prefixed_name_that_already_means_something() {
    // Exact symbols beat prefix splits, so `pallet` with SI prefixes would hide `kpallet`.
    let kilo = unit_text("kpallet", "kg", "7", "");
    let pallet_si = unit_text("pallet", "kg", "1", "prefixes = \"si\"\n");
    // Same file, either order; two files, either order; an alias of a prefixed target.
    for texts in [
        vec![format!("{kilo}{pallet_si}")],
        vec![format!("{pallet_si}{kilo}")],
        vec![kilo.clone(), pallet_si.clone()],
        vec![pallet_si, kilo.clone()],
    ] {
        let configs: Vec<Config> = texts.iter().map(|x| cfg(x, "t.toml")).collect();
        let mut reg = registry().unwrap();
        let err = apply(&mut reg, &configs).unwrap_err();
        assert_eq!(err.code(), "invalid_definition", "{texts:?}");
    }
    // Without prefixes there is nothing to hide, in either order.
    let plain = unit_text("pallet", "kg", "1", "");
    let mut reg = registry().unwrap();
    apply(&mut reg, &[cfg(&format!("{kilo}{plain}"), "t.toml")]).unwrap();
    assert_eq!(factor(&reg, "kpallet", "kg"), Ratio::from_int(7));
    // Against the shipped table: `ol` with SI prefixes would make `mol` a milli-ol.
    let msg = refused(&unit_text("ol", "kg", "1", "prefixes = \"si\"\n"));
    assert!(msg.contains("would hide 'mol'"), "{msg}");
    // A mapping that takes a prefixed target's name is checked the same way.
    let msg = refused("[map]\nmm = \"m\"\n");
    assert!(msg.contains("already means a unit"), "{msg}");
    let mut reg = registry().unwrap();
    let both = cfg(&format!("{kilo}[map]\npallet = \"tonne\"\n"), "t.toml");
    let err = apply(&mut reg, &[both]).unwrap_err();
    assert!(err.to_string().contains("would hide 'kpallet'"), "{err}");
}

#[test]
fn a_config_cannot_turn_an_ambiguous_prefixed_name_into_a_quiet_reading() {
    // `Miton` is mega + the ambiguous `ton`. A mapping `iton` must not make it a mega-iton.
    let base = registry().unwrap();
    assert_eq!(
        base.conversion("Miton", "kg").unwrap_err().code(),
        "ambiguous_symbol"
    );
    for text in [
        "[map]\niton = \"tonne\"\n".to_string(),
        unit_text("iton", "kg", "1", "prefixes = \"si\"\n"),
    ] {
        let mut reg = registry().unwrap();
        apply(&mut reg, &[cfg(&text, "t.toml")]).unwrap();
        assert_eq!(
            reg.conversion("Miton", "kg").unwrap_err().code(),
            "ambiguous_symbol",
            "{text}"
        );
        // The new name itself works.
        assert!(reg.conversion("iton", "kg").is_ok());
    }
}

// ---- bundled configs (ADR-014) ----

fn bundled_registry(names: &[&str]) -> Registry {
    let mut reg = registry().unwrap();
    let parsed: Vec<Config> = names
        .iter()
        .map(|n| dimvexa_config::bundled(n).unwrap().config().unwrap())
        .collect();
    apply(&mut reg, &parsed).unwrap();
    reg
}

#[test]
fn every_bundled_config_applies_on_its_own_and_all_together() {
    for b in dimvexa_config::BUNDLED {
        bundled_registry(&[b.name]);
    }
    let all: Vec<&str> = dimvexa_config::BUNDLED.iter().map(|b| b.name).collect();
    bundled_registry(&all);
}

#[test]
fn us_and_gb_resolve_the_regional_names() {
    let us = bundled_registry(&["us"]);
    assert_eq!(
        factor(&us, "ton", "kg"),
        factor(&us, "ton_short", "kg"),
        "us ton"
    );
    assert_eq!(factor(&us, "mile", "m"), factor(&us, "mi", "m"));
    assert_eq!(factor(&us, "gal", "L"), factor(&us, "gal_us", "L"));
    assert_eq!(factor(&us, "pt", "L"), factor(&us, "pt_us", "L"));
    assert_eq!(factor(&us, "qt", "L"), factor(&us, "qt_us", "L"));
    let gb = bundled_registry(&["gb"]);
    assert_eq!(factor(&gb, "ton", "kg"), factor(&gb, "ton_long", "kg"));
    assert_eq!(factor(&gb, "gal", "L"), factor(&gb, "gal_imp", "L"));
    // The US and the UK gallon differ, so the same name gives different answers per config.
    assert_ne!(factor(&us, "gal", "L"), factor(&gb, "gal", "L"));
    // oz stays refused: mass or volume, in both countries
    for reg in [&us, &gb] {
        assert_eq!(
            reg.conversion("oz", "g").unwrap_err().code(),
            "ambiguous_symbol"
        );
    }
    // later wins: us then gb gives the British ton
    let both = bundled_registry(&["us", "gb"]);
    assert_eq!(factor(&both, "ton", "kg"), factor(&both, "ton_long", "kg"));
}

#[test]
fn metric_trade_and_the_year_configs_resolve_their_names() {
    let mt = bundled_registry(&["metric-trade"]);
    assert_eq!(factor(&mt, "mt", "kg"), Ratio::from_int(1000));
    assert_eq!(factor(&mt, "ton", "kg"), Ratio::from_int(1000));
    let julian = bundled_registry(&["year-julian"]);
    let civil = bundled_registry(&["year-civil"]);
    for name in ["yr", "year"] {
        assert_eq!(factor(&julian, name, "d"), Ratio::new(1461, 4).unwrap());
        assert_eq!(factor(&civil, name, "d"), Ratio::from_int(365));
    }
}

#[test]
fn only_the_water_config_carries_densities() {
    let water = dimvexa_config::bundled("water").unwrap().config().unwrap();
    assert_ne!(water.constants(), []);
    for c in water.constants() {
        assert!(!c.conditions.trim().is_empty() && !c.source.trim().is_empty());
        assert!(c.conditions.contains("air-free"), "{}", c.name);
    }
    for b in dimvexa_config::BUNDLED.iter().filter(|b| b.name != "water") {
        let cfg = b.config().unwrap();
        assert!(cfg.constants().is_empty(), "{} carries a constant", b.name);
    }
    // 25 degC: 997.04702 kg/m3; maximum density: 999.97495 kg/m3
    let get = |n: &str| {
        water
            .constants()
            .iter()
            .find(|c| c.name == n)
            .unwrap()
            .value
    };
    assert_eq!(get("water_25C"), Ratio::new(99_704_702, 100_000).unwrap());
    assert_eq!(
        get("water_max_density"),
        Ratio::new(99_997_495, 100_000).unwrap()
    );
}

#[test]
fn a_density_literal_is_converted_to_kg_per_m3_exactly() {
    use dimvexa_config::Density;
    let reg = registry().unwrap();
    let d = Density::density_from_text(&reg, "998.2067kg/m3").unwrap();
    assert_eq!(d.value, Ratio::new(9_982_067, 10_000).unwrap());
    let g = Density::density_from_text(&reg, "0.7 g/cm3").unwrap();
    assert_eq!(g.value, Ratio::from_int(700));
    let l = Density::density_from_text(&reg, "1 kg/L").unwrap();
    assert_eq!(l.value, Ratio::from_int(1000));
    for (text, want) in [
        ("1e3 kg/m3", Ratio::from_int(1000)),
        ("1E3kg/m3", Ratio::from_int(1000)),
        ("1.5e-1 kg/m3", Ratio::new(3, 20).unwrap()),
        ("+5 kg/m3", Ratio::from_int(5)),
        ("1/2 kg/m3", Ratio::new(1, 2).unwrap()),
    ] {
        assert_eq!(
            Density::density_from_text(&reg, text).unwrap().value,
            want,
            "{text}"
        );
    }
    for bad in [
        "998", "kg/m3", "0 kg/m3", "-1 kg/m3", "5 m", "5 degC", "x kg/m3",
    ] {
        assert!(Density::density_from_text(&reg, bad).is_err(), "{bad}");
    }
}

/// ADR-013: a user unit built on a reference unit keeps the reference kind and its conditions, so
/// `MMscf` is still a standard volume at 60 degF and 14.696 psia, also inside compound units.
#[test]
fn a_unit_built_on_a_reference_unit_keeps_the_reference_and_its_conditions() {
    let reg = with(
        "[[unit]]\nsymbol = \"MMscf\"\nbase = \"scf\"\nfactor = \"1000000\"\n\
         exactness = \"defined\"\nsource = \"test\"\n\
         [[unit]]\nsymbol = \"Mscfd\"\nbase = \"scf/d\"\nfactor = \"1000\"\n\
         exactness = \"defined\"\nsource = \"test\"\n",
    );
    let scf = reg.get("scf").unwrap();
    for symbol in ["MMscf", "Mscfd"] {
        let unit = reg.get(symbol).unwrap();
        assert_eq!(unit.kind(), scf.kind(), "{symbol}");
        assert_eq!(unit.conditions(), scf.conditions(), "{symbol}");
        assert!(unit.conditions().contains("14.696 psia"), "{symbol}");
    }
    // The size is the plain factor, within the reference.
    assert_eq!(factor(&reg, "MMscf", "scf"), Ratio::from_int(1_000_000));
    assert_eq!(factor(&reg, "Mscfd", "scf/d"), Ratio::from_int(1_000));
    // And the reference still protects it: never to an actual or another reference volume.
    for to in ["m3", "Nm3", "ft3"] {
        assert_eq!(
            reg.conversion("MMscf", to).unwrap_err().code(),
            "refused",
            "{to}"
        );
    }
    // A unit built on a plain unit has no conditions.
    let plain = with(
        "[[unit]]\nsymbol = \"pail\"\nbase = \"L\"\nfactor = \"20\"\n\
         exactness = \"defined\"\nsource = \"test\"\n",
    );
    assert_eq!(plain.get("pail").unwrap().conditions(), "");
    // The expression carries the conditions of the unit that has the reference.
    assert!(
        reg.parse("MMscf/h")
            .unwrap()
            .conditions()
            .contains("14.696 psia")
    );
    assert_eq!(reg.parse("m3/h").unwrap().conditions(), "");
    assert_eq!(reg.parse("scf/scf").unwrap().conditions(), "");
}

/// Review of 2026-11-11: a power of 0 cancels a reference. `(scf)^0` is the plain number 1 and a
/// unit built on it is plain, so it can never let an actual volume pass as `scf`; a reference at
/// any other power than 1 cannot be a base.
#[test]
fn a_power_zero_of_a_reference_unit_is_plain_and_cannot_reach_the_reference() {
    let reg = registry().unwrap();
    let one = reg.parse("(scf)^0").unwrap();
    assert_eq!(one.kind().name(), "plain");
    assert_eq!(one.reference_power(), 0);
    assert_eq!(one.conditions(), "");
    assert_eq!(reg.conversion("(Nm3)^0", "1").unwrap().factor(), Ratio::ONE);

    let reg = with(
        "[[unit]]\nsymbol = \"stdref\"\nbase = \"(scf)^0\"\nfactor = \"1\"\n\
         exactness = \"defined\"\nsource = \"test\"\n",
    );
    let stdref = reg.get("stdref").unwrap();
    assert_eq!(stdref.kind().name(), "plain");
    assert_eq!(stdref.conditions(), "");
    for to in ["scf", "Nm3"] {
        assert_eq!(
            reg.conversion("stdref.m3", to).unwrap_err().code(),
            "refused",
            "{to}"
        );
    }
    for base in ["(scf)^2", "1/scf", "Nm3/Nm3.scf"] {
        let text = format!(
            "[[unit]]\nsymbol = \"x\"\nbase = \"{base}\"\nfactor = \"1\"\n\
             exactness = \"defined\"\nsource = \"test\"\n"
        );
        let mut reg = registry().unwrap();
        let outcome = apply(&mut reg, &[cfg(&text, "test.toml")]);
        assert!(outcome.is_err(), "{base}");
    }
}

#[test]
fn a_constant_is_a_density_only_if_its_unit_is_one() {
    let reg = registry().unwrap();
    // Any density unit works: 0.9982067 g/cm3 is 998.2067 kg/m3.
    let cfg_g = cfg(
        &WATER
            .replace("9982067/10000", "9982067/10000000")
            .replace("kg/m3", "g/cm3"),
        "g.toml",
    );
    let d = cfg_g.constants()[0].to_density(&reg).unwrap();
    assert_eq!(d.value, Ratio::new(9_982_067, 10_000).unwrap());
    // A constant of another dimension is refused, not guessed.
    let planck = cfg_g.constants()[0].clone();
    let not_density = dimvexa_config::Constant {
        unit: "J.s".to_string(),
        ..planck
    };
    let err = not_density.to_density(&reg).unwrap_err();
    assert_eq!(err.code(), "refused", "{err}");
    assert!(err.to_string().contains("not a density"), "{err}");
    // An unknown unit is an error of the registry, not a silent pass.
    let unknown = dimvexa_config::Constant {
        unit: "Zorp/m3".to_string(),
        ..not_density
    };
    assert!(unknown.to_density(&reg).is_err());
}

#[test]
fn a_config_cannot_redefine_a_built_in_constant() {
    let builtin: Vec<&str> = dimvexa_data::CONSTANTS.iter().map(|c| c.name).collect();
    assert!(builtin.contains(&"pi") && builtin.contains(&"N_A"));
    for name in &builtin {
        let c = cfg(&WATER.replace("water_20C", name), "mine.toml");
        let err = dimvexa_config::reject_builtin_constants(&c, &builtin).unwrap_err();
        assert!(
            err.to_string().contains("cannot be redefined"),
            "{name}: {err}"
        );
    }
    let ok = cfg(WATER, "mine.toml");
    assert!(dimvexa_config::reject_builtin_constants(&ok, &builtin).is_ok());
}

#[test]
fn load_builds_the_constants_and_densities_in_one_place() {
    let builtin = dimvexa_data::CONSTANTS;
    let reg = registry().unwrap();
    let loaded = dimvexa_config::load(reg.clone(), &[cfg(WATER, "w.toml")], builtin).unwrap();
    let names: Vec<&str> = loaded.constants.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["pi", "h", "e", "k_B", "N_A", "water_20C"]);
    assert_eq!(loaded.densities.len(), 1);
    // A named density, a literal, and the two refusals.
    assert_eq!(
        loaded.resolve_density("water_20C").unwrap().value,
        Ratio::new(9_982_067, 10_000).unwrap()
    );
    assert_eq!(
        loaded.resolve_density("0.7 g/cm3").unwrap().value,
        Ratio::from_int(700)
    );
    for bad in ["h", "nope"] {
        assert!(
            matches!(
                loaded.resolve_density(bad),
                Err(dimvexa_config::DensityError::Unknown { .. })
            ),
            "{bad}"
        );
    }
    // A built-in name and an unknown unit are refused at load; a unit from a later config works.
    let pi = cfg(&WATER.replace("water_20C", "pi"), "pi.toml");
    assert!(dimvexa_config::load(reg.clone(), &[pi], builtin).is_err());
    let unit = cfg(
        "[[unit]]\nsymbol = \"blob\"\nbase = \"g\"\nfactor = \"7\"\nexactness = \"defined\"\nsource = \"s\"\n",
        "u.toml",
    );
    let c = cfg(&WATER.replace("kg/m3", "blob/m3"), "c.toml");
    assert!(dimvexa_config::load(reg.clone(), std::slice::from_ref(&c), builtin).is_err());
    assert!(dimvexa_config::load(reg, &[unit, c], builtin).is_ok());
}

#[test]
fn a_config_that_replaces_an_entry_of_an_earlier_one_gets_a_warning() {
    let builtin = dimvexa_data::CONSTANTS;
    let reg = registry().unwrap();
    let (a, b) = (
        cfg(WATER, "a.toml"),
        cfg(&WATER.replace("9982067/10000", "1000"), "b.toml"),
    );
    let loaded = dimvexa_config::load(reg.clone(), &[a.clone(), b], builtin).unwrap();
    assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
    assert!(
        loaded.warnings[0]
            .contains("constant 'water_20C' (b.toml:1) replaces the one from a.toml:1")
    );
    // The later one wins; the same config twice, and distinct names, are silent.
    assert_eq!(loaded.densities[0].value, Ratio::from_int(1000));
    let twice = dimvexa_config::load(reg, &[a.clone(), a], builtin).unwrap();
    assert_eq!(twice.warnings, Vec::<String>::new());
}
