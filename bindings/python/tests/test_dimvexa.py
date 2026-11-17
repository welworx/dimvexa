import math

import pytest

import dimvexa

np = pytest.importorskip("numpy")


def test_scalar_exact_factor() -> None:
    assert dimvexa.convert(10.0, "m/s", "km/h") == 36.0
    assert dimvexa.conversion("m/s", "km/h").factor == "18/5"


def test_angles_are_exact_with_pi_symbolic() -> None:
    # ADR-018: the degree is exactly pi/180 rad, so the factor reads `1/180 pi` and the exact
    # path rounds once.
    assert dimvexa.conversion("deg", "rad").factor == "1/180 pi"
    assert dimvexa.convert(180.0, "deg", "rad", exact=True) == math.pi


def test_affine() -> None:
    assert dimvexa.convert(100.0, "degC", "degF") == pytest.approx(212.0)
    assert dimvexa.conversion("degC", "K").shape == "affine"


def test_exact_path() -> None:
    assert dimvexa.convert(0.1, "m", "mm", exact=True) == 100.0


def test_numpy_array_keeps_shape() -> None:
    a = np.arange(6, dtype=np.float64).reshape(2, 3)
    out = dimvexa.conversion("m", "km")(a)
    assert out.shape == (2, 3)
    assert np.allclose(out, a / 1000)


@pytest.mark.parametrize("order", ["C", "F", "strided"])
def test_numpy_array_layouts_match_the_scalar_path(order: str) -> None:
    c = dimvexa.conversion("degC", "degF")
    base = np.linspace(-40.0, 100.0, 24).reshape(4, 6)
    a = base[:, ::2] if order == "strided" else np.asarray(base, order=order)
    out = c(a)
    assert out.shape == a.shape
    assert [float(v) for v in out.ravel()] == [c(float(v)) for v in a.ravel()]


def test_list_and_inverse() -> None:
    c = dimvexa.conversion("m/s", "km/h")
    assert c([1.0, 2.0]) == [3.6, 7.2]
    assert math.isclose(c.inverse()(3.6), 1.0)


def test_ambiguity_is_a_loud_error_with_a_code() -> None:
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.convert(1.0, "gal", "L")
    assert e.value.code == "ambiguous_symbol"
    assert e.value.candidates == ["gal_us", "gal_imp"]
    assert isinstance(e.value, ValueError)


def test_dimension_mismatch() -> None:
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.convert(1.0, "m", "s")
    assert e.value.code == "dimension_mismatch"


def test_quantity_kinds() -> None:
    t = dimvexa.quantity("100 degC")
    assert t.to("degF").to_float() == 212.0
    with pytest.raises(dimvexa.DimvexaError) as e:
        t.to("delta_K")
    assert e.value.code == "kind_mismatch"


def test_quantity_to() -> None:
    q = dimvexa.quantity("1 km").to("m")
    assert q.to_float() == 1000.0


def test_unit_table() -> None:
    assert "m" in dimvexa.units()
    info = dimvexa.unit_info("m")
    assert info is not None
    assert info["symbol"] == "m"
    assert info["status"] == "current"
    legacy = dimvexa.unit_info("chain_us_survey")
    assert legacy is not None  # ADR-026
    assert legacy["status"] == "legacy"
    assert dimvexa.unit_info("nope") is None


def test_gauge_pressures_and_gas_volumes_are_not_implemented() -> None:
    # ADR-021, ADR-023: they need an ambient pressure or a gas model; the error says so.
    for unit in ["psig", "kPag", "Nm3", "scf"]:
        with pytest.raises(dimvexa.DimvexaError) as err:
            dimvexa.convert(1.0, unit, "bar" if unit.endswith("g") else "m3")
        assert err.value.code == "refused"
        assert "not implemented" in str(err.value)
    assert dimvexa.convert(1.0, "bbl", "L") == 158.987294928
    assert dimvexa.data_version().startswith("v")


WATER_TEXT = (
    '[[constant]]\nid = "rho"\nname = "rho"\nvalue = "0.9982067"\nunit = "g/cm3"\nexactness = "measured"\n'
    'conditions = "c"\nsource = "s"\n'
)


def test_density_and_configs_like_the_cli() -> None:
    # A shipped config resolves an ambiguous name; a density is used only when it is needed.
    assert dimvexa.convert(1.0, "ton", "kg", configs=["us-customary"]) == 907.18474
    assert dimvexa.convert(1.0, "L", "kg", density="998.2067kg/m3") == 0.9982067
    assert dimvexa.convert(1.0, "L", "kg", density="water_25C", configs=["water"]) == 0.99704702
    assert dimvexa.convert(1.0, "L", "kg", config_texts=[WATER_TEXT], density="rho") == 0.9982067
    assert dimvexa.convert(1.0, "L", "mL", density="998kg/m3") == 1000.0
    assert dimvexa.conversion("L", "kg", density="998.2067kg/m3")(1.0) == 0.9982067
    with pytest.raises(dimvexa.DimvexaError) as err:
        dimvexa.convert(1.0, "L", "kg")
    assert err.value.code == "dimension_mismatch"
    for bad in ("nope", "h_P"):  # unknown, and a constant that is not a density
        with pytest.raises(dimvexa.DimvexaError):
            dimvexa.convert(1.0, "L", "kg", density=bad)
    with pytest.raises(dimvexa.DimvexaError):
        dimvexa.convert(1.0, "m", "m", configs=["nope"])


def test_constants_table() -> None:
    names = [c["id"] for c in dimvexa.constants()]
    assert names == ["pi", "h_P", "e", "k_B", "N_A", "c", "R"]
    h = dimvexa.constants()[1]
    assert h["name"] == "Planck constant"
    assert (h["value"], h["value_text"], h["unit"]) == ("662607015 10^-42", "6.62607015e-34", "J.s")
    assert h["exactness"] == "defined" and h["file"] == "built-in"
    both = [c["id"] for c in dimvexa.constants(configs=["water"], config_texts=[WATER_TEXT])]
    assert both[-3:] == ["water_25C", "water_max_density", "rho"]


def test_a_config_cannot_redefine_a_builtin_constant() -> None:
    pi = WATER_TEXT.replace("rho", "pi")
    with pytest.raises(dimvexa.DimvexaError) as err:
        dimvexa.constants(config_texts=[pi])
    assert err.value.code == "invalid_definition"
    assert "cannot be redefined" in str(err.value)
    # A unit defined by another loaded config is fine; an unknown unit is a loud error.
    unit = '[[unit]]\nid = "blob"\nbase = "g"\nfactor = "7"\nexactness = "defined"\nsource = "s"\n'
    const = WATER_TEXT.replace("g/cm3", "blob/m3")
    assert dimvexa.constants(config_texts=[unit, const])[-1]["unit"] == "blob/m3"
    with pytest.raises(dimvexa.DimvexaError):
        dimvexa.constants(config_texts=[const])


def test_replacing_an_entry_of_an_earlier_config_warns() -> None:
    mine = WATER_TEXT.replace("rho", "water_25C")
    with pytest.warns(
        UserWarning, match="constant 'water_25C'.*replaces the one from bundled:water"
    ):
        dimvexa.constants(configs=["water"], config_texts=[mine])


def test_config_files_must_be_regular_files_and_an_empty_density_is_none(tmp_path) -> None:  # type: ignore[no-untyped-def]
    # A directory is not a regular file (on every platform; /dev/zero does not exist on Windows).
    with pytest.raises(dimvexa.DimvexaError, match="not a regular file"):
        dimvexa.convert(1.0, "m", "m", configs=[str(tmp_path)])
    assert dimvexa.convert(1.0, "m", "km", density="") == 0.001


UNIT_TEXT = '[[unit]]\nid = "blob"\nbase = "g"\nfactor = "7"\nexactness = "defined"\nsource = "s"\n'


def test_quantity_takes_configs_and_density() -> None:
    q = dimvexa.quantity("2 ton", configs=["us-customary"]).to("kg")
    assert q.to_float() == 1814.36948
    # A density turns volume into mass exactly, and only when asked for.
    v = dimvexa.quantity("1 L", density="998.2067kg/m3")
    assert v.to("kg").to_float() == 0.9982067
    assert v.to("kg").to("L").to_float() == 1.0
    assert v.to("mL").to_float() == 1000.0
    with pytest.raises(dimvexa.DimvexaError):
        dimvexa.quantity("1 L").to("kg")
    # Products keep the context.
    m = (dimvexa.quantity("2", density="998kg/m3") * dimvexa.quantity("1 L")).to("kg")
    assert m.to_float() == 1.996
    with pytest.raises(dimvexa.DimvexaError):
        dimvexa.quantity("2 blob")


def test_units_and_unit_info_list_config_units() -> None:
    assert "blob" not in dimvexa.units()
    assert "blob" in dimvexa.units(config_texts=[UNIT_TEXT])
    info = dimvexa.unit_info("blob", config_texts=[UNIT_TEXT])
    assert info is not None
    assert info["factor"] == "7/1000" and info["defined_in"] == "config-text-1:1"
    assert dimvexa.unit_info("blob") is None
    assert dimvexa.unit_info("m")["defined_in"] is None  # type: ignore[index]
    assert dimvexa.unit_info("m", configs=["us-customary"])["defined_in"] is None  # type: ignore[index]
    # A name a config maps resolves to its unit.
    assert dimvexa.unit_info("mt", config_texts=['[map]\nmt = "tonne"\n']) is not None


def test_a_constant_cannot_be_named_like_a_unit() -> None:
    for bad in ("h", "ton"):  # the hour, and an ambiguous bare symbol
        with pytest.raises(dimvexa.DimvexaError, match="also a unit"):
            dimvexa.constants(config_texts=[WATER_TEXT.replace("rho", bad)])


def test_a_string_value_may_contain_pi_and_stays_exact() -> None:
    assert dimvexa.convert("4pi", "rad", "deg") == 720.0
    assert dimvexa.convert("pi/2", "rad", "deg") == 90.0
    assert dimvexa.convert(" 2*pi^1 ", "rad", "deg") == 360.0
    assert dimvexa.convert("98.6", "degF", "degC") == 37.0
    assert dimvexa.conversion("rad", "deg")("4pi") == 720.0
    q = dimvexa.quantity("pi/2 rad")
    assert q.value == "1/2 pi"
    assert q.to("deg").to_float() == 90.0
    assert dimvexa.quantity("4 pi rad", configs=["us-customary"]).to("deg").to_float() == 720.0
    assert dimvexa.quantity("2.5 km").to("m").to_float() == 2500.0


@pytest.mark.parametrize(
    ("value", "code"),
    [
        ("1/2pi", "ambiguous_expression"),
        ("2*h_P", "refused"),
        ("tau", "invalid_number"),
        ("1/0", "division_by_zero"),
    ],
)
def test_a_bad_string_value_is_a_loud_error(value: str, code: str) -> None:
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.convert(value, "rad", "deg")
    assert e.value.code == code


def test_pi_into_an_offset_unit_is_refused() -> None:
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.convert("pi", "degC", "degF")
    assert e.value.code == "refused"


def test_calc_is_exact_when_the_mathematics_is() -> None:
    r = dimvexa.calc("sqrt(9 m2)")
    assert r == {
        "value": 3.0,
        "unit": "m",
        "exact": True,
        "rounded": False,
        "fraction": "3/1",
        "dimension": [0, 1, 0, 0, 0, 0, 0],
        "kind": "plain",
        "text": "3 m",
    }
    assert dimvexa.calc("100 kg * 1 g_n * sin(30 deg)", to="N")["text"] == "490.3325 N"
    # a plain number has no unit
    plain = dimvexa.calc("3 m / 2 m")
    assert (plain["value"], plain["unit"], plain["fraction"], plain["text"]) == (
        1.5,
        "",
        "3/2",
        "1.5",
    )
    assert plain["dimension"] == [0] * 7


def test_calc_approximate_results_say_so() -> None:
    r = dimvexa.calc("sqrt(2 m2)")
    assert r["exact"] is False
    assert r["fraction"] is None
    assert r["text"] == "≈ 1.4142135623730951 m"


def test_calc_named_values_equal_inline_quantities() -> None:
    inline = dimvexa.calc("0.61 * 10 cm2 * sqrt(2 * 50 kPa / 998 kg/m3)", to="L/s")
    named = dimvexa.calc(
        "Cd * A * sqrt(2 * dp / rho)",
        vars={"Cd": "0.61", "A": "10 cm2", "dp": "50 kPa", "rho": "998 kg/m3"},
        to="L/s",
    )
    assert inline == named
    assert inline["text"] == "≈ 6.106109165276735 L/s"
    # x^0.5, y^(1/2) and sqrt(x) agree
    for e in ("x^0.5", "x^(1/2)", "sqrt(x)"):
        assert dimvexa.calc(e, vars={"x": "9 m2"})["text"] == "3 m"


def test_calc_min_max_need_one_dimension_and_keep_the_winners_unit() -> None:
    assert dimvexa.calc("min(3 ft, 4 m)")["text"] == "3 ft"
    assert dimvexa.calc("max(4 m, 4 km)")["text"] == "4 km"
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.calc("min(4 m, 3 L)")
    assert e.value.code == "dimension_mismatch"


def test_calc_errors_carry_codes_and_configs_apply() -> None:
    for expr, code in [
        ("1 m + 1 s", "dimension_mismatch"),
        ("floor(1.5 m)", "refused"),
        ("log(100)", "refused"),
        ("1 +", "parse_error"),
    ]:
        with pytest.raises(dimvexa.DimvexaError) as e:
            dimvexa.calc(expr)
        assert e.value.code == code, expr
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.calc("1 m", to="s")
    assert e.value.code == "dimension_mismatch"
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.calc("1 ton * 1 g_n", to="N")
    assert e.value.code == "ambiguous_symbol"
    r = dimvexa.calc("1 ton * 1 g_n", to="N", configs=["us-customary"])
    assert r["text"] == "8896.443230521 N"


def test_calc_refuses_bad_names_and_pi() -> None:
    for name in ("d p", " x ", "1x", "pi"):
        with pytest.raises(dimvexa.DimvexaError) as e:
            dimvexa.calc("1 m", vars={name: "3 m"})
        assert e.value.code == "invalid_definition", name


def test_calc_number_text_is_the_text_of_the_command_line() -> None:
    # the same forms as format_f64 in dimvexa-calc, which the CLI prints too
    for expr, text in [
        ("1 m / 3", "0.3333333333333333… m"),
        ("1e15 m", "1000000000000000 m"),
        ("1e16 m", "1e16 m"),
        ("0.0001 m", "0.0001 m"),
        ("0.00001 m", "1e-5 m"),
        ("2 m - 2 m", "0 m"),
        ("-0.5 m", "-0.5 m"),
        ("sqrt(1e40 m2)", "1e20 m"),
        ("sqrt(2e-10 m2)", "≈ 1.4142135623730951e-5 m"),
    ]:
        assert dimvexa.calc(expr)["text"] == text, expr
    t = dimvexa.calc("20 degC + 5 K")
    assert (t["text"], t["kind"]) == ("25 degC", "absolute_temperature")


def test_calc_marks_rounding_prints_beyond_f64_and_refuses_ambiguous_names() -> None:
    third = dimvexa.calc("1 m / 3")
    assert (third["rounded"], third["exact"], third["fraction"]) == (True, True, "1/3")
    assert dimvexa.calc("0.1 + 0.2")["rounded"] is False
    huge = dimvexa.calc("1e300 * 1e300 m")
    assert (huge["value"], huge["text"], huge["fraction"]) == (None, "1e600 m", "10^600")
    # a name that is also a unit in the equation is refused, not guessed
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.calc("0.5 h", vars={"h": "3 m"})
    assert e.value.code == "invalid_definition"
    assert dimvexa.calc("h * 2", vars={"h": "3 m"})["text"] == "6 m"
    for bad in ("sin(30 deg * 30 deg)", "sin(1 / 1 rad)"):
        with pytest.raises(dimvexa.DimvexaError) as e:
            dimvexa.calc(bad)
        assert e.value.code == "refused", bad


def test_calc_knows_the_constants_tan_atan2_and_merges_units() -> None:
    assert dimvexa.calc("k_B * 300 K")["text"] == "4.141947e-21 J"
    assert dimvexa.calc("tan(45 deg)")["text"] == "1"
    assert dimvexa.calc("atan2(1 m, 100 cm)", to="deg")["text"] == "45 deg"
    assert dimvexa.calc("1 m * 1 m")["unit"] == "m^2"
    with pytest.warns(UserWarning, match="replaces the built-in constant"):
        assert dimvexa.calc("e * 2", vars={"e": "3 C"})["text"] == "6 C"


def test_calc_config_constants_kelvin_pi_names_and_notation() -> None:
    r = dimvexa.calc("1 L * water_25C", to="kg", configs=["water"])
    assert r["text"] == "0.99704702 kg"
    gas = dimvexa.calc(
        "n * R * kelvin(T) / V", vars={"n": "2 mol", "T": "20 degC", "V": "50 L"}, to="kPa"
    )
    assert gas["text"].startswith("97.4953886604649")
    assert dimvexa.calc("2 pi r", vars={"r": "3 m"})["text"] == "18.84955592153876… m"
    assert dimvexa.calc("x^n", vars={"x": "9 m2", "n": "0.5"})["text"] == "3 m"
    assert dimvexa.calc("10 m² · 3 m")["text"] == "30 m^3"


def test_calc_r_and_r2_are_two_names_and_an_unknown_name_is_said_so() -> None:
    both = {"r": "3 m", "r2": "5 m"}
    assert dimvexa.calc("2 r2 + r", vars=both)["text"] == "13 m"
    assert dimvexa.calc("2 r^2 / r2", vars=both)["text"] == "3.6 m"
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.calc("2 r2", vars=both)  # r is not used
    assert e.value.code == "invalid_definition"
    assert "the variable 'r' is not used" in str(e.value)
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.calc("2 r")
    assert e.value.code == "parse_error"
    assert "not a unit and not a variable" in str(e.value)
    assert dimvexa.calc("2 r", vars={"r": "3 m"})["text"] == "6 m"


def test_calc_review_round_three_cases() -> None:
    assert dimvexa.calc("-40 degC")["text"] == "-40 degC"
    assert dimvexa.calc("10^-400")["text"] == "1e-400"
    assert dimvexa.calc("1^128")["text"] == "1"
    assert dimvexa.calc("sin(179.9999999 deg)")["text"] == "≈ 1.7453292519943295e-9"
    for expr in ("x^2 n",):
        with pytest.raises(dimvexa.DimvexaError) as e:
            dimvexa.calc(expr, vars={"x": "10 m", "n": "2"})
        assert e.value.code == "parse_error"
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.calc("1 m", vars={"2x": "1 m"})
    assert e.value.code == "invalid_definition"
