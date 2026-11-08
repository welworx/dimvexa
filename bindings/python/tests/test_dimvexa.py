import math

import dimvexa
import pytest

np = pytest.importorskip("numpy")


def test_scalar_exact_factor():
    assert dimvexa.convert(10.0, "m/s", "km/h") == 36.0
    assert dimvexa.conversion("m/s", "km/h").factor == "18/5"


def test_affine():
    assert dimvexa.convert(100.0, "degC", "degF") == pytest.approx(212.0)
    assert dimvexa.conversion("degC", "K").shape == "affine"


def test_exact_path():
    assert dimvexa.convert(0.1, "m", "mm", exact=True) == 100.0


def test_numpy_array_keeps_shape():
    a = np.arange(6, dtype=np.float64).reshape(2, 3)
    out = dimvexa.conversion("m", "km")(a)
    assert out.shape == (2, 3)
    assert np.allclose(out, a / 1000)


def test_list_and_inverse():
    c = dimvexa.conversion("m/s", "km/h")
    assert c([1.0, 2.0]) == [3.6, 7.2]
    assert math.isclose(c.inverse()(3.6), 1.0)


def test_ambiguity_is_a_loud_error_with_a_code():
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.convert(1.0, "gal", "L")
    assert e.value.code == "ambiguous_symbol"
    assert e.value.candidates == ["gal_us", "gal_imp"]
    assert isinstance(e.value, ValueError)


def test_dimension_mismatch():
    with pytest.raises(dimvexa.DimvexaError) as e:
        dimvexa.convert(1.0, "m", "s")
    assert e.value.code == "dimension_mismatch"


def test_quantity_kinds():
    t = dimvexa.quantity("20 degC")
    assert str(t + dimvexa.quantity("5 delta_K")) == "25 degC"
    with pytest.raises(dimvexa.DimvexaError) as e:
        t + dimvexa.quantity("5 K")
    assert e.value.code == "kind_mismatch"
    assert e.value.suggestions == ["delta_K", "delta_degC"]


def test_quantity_to():
    q = dimvexa.quantity("1 km").to("m")
    assert q.to_float() == 1000.0


def test_unit_table():
    assert "m" in dimvexa.units()
    info = dimvexa.unit_info("m")
    assert info is not None and info["symbol"] == "m"
    assert dimvexa.unit_info("nope") is None
    assert dimvexa.data_version().startswith("v")
