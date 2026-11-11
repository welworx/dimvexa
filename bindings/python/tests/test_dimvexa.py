import math

import pytest

import dimvexa

np = pytest.importorskip("numpy")


def test_scalar_exact_factor() -> None:
    assert dimvexa.convert(10.0, "m/s", "km/h") == 36.0
    assert dimvexa.conversion("m/s", "km/h").factor == "18/5"


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
