# Data sources and licensing policy

- Code: `MIT OR Apache-2.0`.
- Unit data is factual (definitions and published constants). Each unit records `source` and `note`.
- Allowed sources: SI Brochure, NIST SP 811 and SP 330, NIST Handbook 44 and 130, CODATA, ISO 80000 and statutory definitions.
- Oracles only, never copied: GNU units and rink (GPL).
- Citations: checked against the primary texts on 2026-11-09; the result, the wording to tighten and the few items whose text could not be read are in [`source-verification.md`](source-verification.md). Golden vectors catch arithmetic errors, not wrong citations.
- UCUM and UDUNITS licence status is still to be researched (see plan section 12).

## Disclaimer, independence and benchmarked projects

The files in this repository are provided **"as is"**, without warranty of any kind, express or
implied, including but not limited to merchantability, fitness for a particular purpose,
accuracy, completeness and non-infringement. The author and contributors accept no liability
for any claim, damage, loss or other consequence arising from the use of, or inability to use,
this software, its unit tables or its documentation, and give no indemnity.

Unit factors, sources and benchmark figures can contain errors; verify any result you depend on
against the primary source. Do not use this software as the only safeguard in safety-critical,
medical, legal, financial, custody-transfer or similar settings.

I do not accept any responsibility for any harmful, unlawful or otherwise improper use of the
material in this repository. You alone are responsible for complying with the laws and
regulations that apply to you. The licence texts ([`LICENSE-MIT`](LICENSE-MIT),
[`LICENSE-APACHE`](LICENSE-APACHE)) contain the binding terms and prevail over this summary.

**Licence.** `MIT OR Apache-2.0`, at your option. Commercial use is allowed. Please keep the
copyright notice and the [`NOTICE`](NOTICE) file with copies, and cite the project when you use
it in research (see [`CITATION.cff`](CITATION.cff)). Contributions are licensed under the same
terms (see [`CONTRIBUTING.md`](CONTRIBUTING.md)). Data provenance and policy:
[`docs/licensing.md`](docs/licensing.md).

**Independence.** Dimvexa is an independent project. It is not affiliated with, associated
with, sponsored by or endorsed by any of the projects, companies or organisations named in this
repository. All product and project names are the property of their respective owners and are
used for identification only. The benchmark results describe the versions, inputs and machine
recorded in [`docs/benchmarks/comparison/comparison.md`](docs/benchmarks/comparison/comparison.md) and are not a
statement about those projects in general. Maintainers can ask for a runner correction through
the *Benchmark runner review* issue form.

**Libraries benchmarked** (method in [`docs/benchmarking.md`](docs/benchmarking.md)):

- [Astropy (`astropy.units`)](https://github.com/astropy/astropy)
- [convert-units](https://github.com/convert-units/convert-units)
- [js-quantities](https://github.com/gentooboontoo/js-quantities)
- [math.js](https://github.com/josdejong/mathjs)
- [Pint](https://github.com/hgrecco/pint)
- [unyt](https://github.com/yt-project/unyt)
- [uom (Rust, units of measurement)](https://github.com/iliekturtles/uom)

Plain-loop and NumPy "ceilings" are included as reference lines only. The unit table is built
from primary definitions (BIPM SI Brochure, NIST SP 811 and similar); no data is copied from
GNU units or rink.
