from typing import Any

__version__: str

class DimvexaError(ValueError):
    code: str
    suggestions: list[str]
    candidates: list[str]

class Conversion:
    @property
    def shape(self) -> str: ...
    @property
    def factor(self) -> str: ...
    @property
    def offset(self) -> str: ...
    def __call__(self, x: Any) -> Any: ...
    def apply_exact(self, x: float) -> float: ...
    def inverse(self) -> Conversion: ...

class Quantity:
    @property
    def value(self) -> str: ...
    @property
    def unit(self) -> str: ...
    def to(self, unit: str) -> Quantity: ...
    def to_float(self) -> float: ...
    def __mul__(self, other: Quantity) -> Quantity: ...
    def __truediv__(self, other: Quantity) -> Quantity: ...

def convert(
    value: Any,  # float, sequence, ndarray, or str: "4pi", "pi/2", "0.1" (exact)
    from_unit: str,
    to_unit: str,
    *,
    exact: bool = False,
    density: str | None = None,
    configs: list[str] | None = None,
    config_texts: list[str] | None = None,
) -> Any: ...
def conversion(
    from_unit: str,
    to_unit: str,
    *,
    density: str | None = None,
    configs: list[str] | None = None,
    config_texts: list[str] | None = None,
) -> Conversion: ...
def constants(
    *, configs: list[str] | None = None, config_texts: list[str] | None = None
) -> list[dict[str, Any]]: ...
def quantity(
    text: str,
    *,
    density: str | None = None,
    configs: list[str] | None = None,
    config_texts: list[str] | None = None,
) -> Quantity: ...
def units(
    *, configs: list[str] | None = None, config_texts: list[str] | None = None
) -> list[str]: ...
def unit_info(
    symbol: str,
    *,
    configs: list[str] | None = None,
    config_texts: list[str] | None = None,
) -> dict[str, Any] | None: ...
def data_version() -> str: ...
