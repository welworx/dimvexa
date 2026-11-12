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
    assert info["conditions"] is None
    assert dimvexa.unit_info("nope") is None


def test_reference_state_units() -> None:
    # ADR-013: gauge pressures and gas volumes carry their reference state as `conditions`.
    nm3 = dimvexa.unit_info("Nm3")
    assert nm3 is not None
    assert nm3["kind"] == "normal_volume"
    assert "273.15 K" in nm3["conditions"]
    assert "101 325 Pa" in nm3["conditions"]
    scf = dimvexa.unit_info("scf")
    assert scf is not None
    assert "14.696 psia" in scf["conditions"]
    psig = dimvexa.unit_info("psig")
    assert psig is not None
    assert psig["kind"] == "gauge_pressure"
    assert psig["conditions"]
    # Same reference converts, a different one is refused, never guessed.
    assert dimvexa.convert(60.0, "Nm3/h", "Nm3/min") == 1.0
    assert dimvexa.convert(1.0, "bbl", "L") == 158.987294928
    for a, b in [("Nm3", "m3"), ("scf", "Nm3"), ("psig", "bar")]:
        with pytest.raises(dimvexa.DimvexaError) as err:
            dimvexa.convert(1.0, a, b)
        assert err.value.code == "refused"
    assert dimvexa.data_version().startswith("v")


WATER_TEXT = (
    '[[constant]]\nid = "rho"\nvalue = "0.9982067"\nunit = "g/cm3"\nexactness = "measured"\n'
    'conditions = "c"\nsource = "s"\n'
)


def test_density_and_configs_like_the_cli() -> None:
    # A shipped config resolves an ambiguous name; a density is used only when it is needed.
    assert dimvexa.convert(1.0, "ton", "kg", configs=["us"]) == 907.18474
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
    assert names == ["pi", "h_P", "e", "k_B", "N_A"]
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
    unit = '[[unit]]\nsymbol = "blob"\nbase = "g"\nfactor = "7"\nexactness = "defined"\nsource = "s"\n'
    const = WATER_TEXT.replace("g/cm3", "blob/m3")
    assert dimvexa.constants(config_texts=[unit, const])[-1]["unit"] == "blob/m3"
    with pytest.raises(dimvexa.DimvexaError):
        dimvexa.constants(config_texts=[const])


def test_replacing_an_entry_of_an_earlier_config_warns() -> None:
    mine = WATER_TEXT.replace("rho", "water_25C")
    with pytest.warns(UserWarning, match="constant 'water_25C'.*replaces the one from bundled:water"):
        dimvexa.constants(configs=["water"], config_texts=[mine])


def test_config_files_must_be_regular_files_and_an_empty_density_is_none() -> None:
    with pytest.raises(dimvexa.DimvexaError, match="not a regular file"):
        dimvexa.convert(1.0, "m", "m", configs=["/dev/zero"])
    assert dimvexa.convert(1.0, "m", "km", density="") == 0.001


UNIT_TEXT = (
    '[[unit]]\nsymbol = "blob"\nbase = "g"\nfactor = "7"\nexactness = "defined"\nsource = "s"\n'
)


def test_quantity_takes_configs_and_density() -> None:
    q = dimvexa.quantity("2 ton", configs=["us"]).to("kg")
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
    assert dimvexa.unit_info("m", configs=["us"])["defined_in"] is None  # type: ignore[index]
    # A name a config maps resolves to its unit.
    assert dimvexa.unit_info("mt", config_texts=['[map]\nmt = "tonne"\n']) is not None


def test_a_constant_cannot_be_named_like_a_unit() -> None:
    for bad in ("h", "ton"):  # the hour, and an ambiguous bare symbol
        with pytest.raises(dimvexa.DimvexaError, match="also a unit"):
            dimvexa.constants(config_texts=[WATER_TEXT.replace("rho", bad)])
