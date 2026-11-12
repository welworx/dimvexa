#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Offline tests for gen_corpus.py and report.py (standard library only):

    python3 -I benchmarks/tools/test_report.py
"""
import json
import subprocess
import sys
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
BENCH = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import check_versions  # noqa: E402
import report  # noqa: E402


class ReportOnFixtures(unittest.TestCase):
    def run_report(self, results, out):
        return report.main(["--results", str(results), "--out", str(out)])

    def test_fixture_report_is_labelled_and_complete(self):
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / "comparison.md"
            self.assertEqual(self.run_report(BENCH / "fixtures" / "results", out), 0)
            text = out.read_text()
            self.assertIn("FIXTURE DATA", text)
            for h in ("## Speed", "## Accuracy", "## Coverage and safety matrix", "Per-request outcome", "### Unit coverage"):
                self.assertIn(h, text)
            self.assertIn("ACCEPTED", text)
            svgs = list((Path(d) / "img").glob("*.svg"))
            self.assertGreaterEqual(len(svgs), 6)
            for s in svgs:
                t = s.read_text()
                self.assertTrue(t.startswith("<svg") and t.rstrip().endswith("</svg>"))
                self.assertIn("FIXTURE DATA", t)

    def test_dimvexa_bars_have_their_own_color(self):
        with tempfile.TemporaryDirectory() as d:
            report.bar_chart(Path(d) / "c.svg", "t", [("dimvexa-py 0.1 / conv", 1.0), ("pint 0.26 / plain", 5.0)], "ns")
            t = (Path(d) / "c.svg").read_text()
            self.assertEqual(t.count(f'fill="{report.DIMVEXA_COLOR}"'), 1)
            self.assertEqual(t.count(f'fill="{report.OTHER_COLOR}"'), 1)

    def test_fixture_cannot_overwrite_official_page(self):
        with self.assertRaises(SystemExit):
            self.run_report(BENCH / "fixtures" / "results", report.OFFICIAL)

    def test_empty_results_write_a_notice_not_a_table(self):
        with tempfile.TemporaryDirectory() as d:
            empty = Path(d) / "none"
            empty.mkdir()
            out = Path(d) / "c.md"
            self.run_report(empty, out)
            self.assertIn("No benchmark run has been published yet", out.read_text())

    def test_mixing_fixtures_and_real_results_is_refused(self):
        with tempfile.TemporaryDirectory() as d:
            for f in (BENCH / "fixtures" / "results").glob("*.json"):
                (Path(d) / f.name).write_text(f.read_text())
            real = json.loads(next((BENCH / "fixtures" / "results").glob("*.json")).read_text())
            real["fixture"] = False
            (Path(d) / "real.json").write_text(json.dumps(real))
            with self.assertRaises(SystemExit):
                self.run_report(d, Path(d) / "o" / "c.md")


class Corpus(unittest.TestCase):
    def test_committed_corpus_is_reproducible(self):
        r = subprocess.run([sys.executable, "-I", str(TOOLS / "gen_corpus.py"), "--check"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_every_must_convert_case_has_exact_truth_and_source(self):
        cases = json.loads((BENCH / "corpus" / "must_convert.json").read_text())["cases"]
        self.assertGreater(len(cases), 100)
        for c in cases:
            self.assertRegex(c["expect_exact"], r"^-?\d+/\d+$")
            self.assertTrue(c["source"])
            self.assertEqual(float(c["expect_f64"]), float(Fraction(c["expect_exact"])))

    def test_refuse_list_names_the_required_cases(self):
        cases = json.loads((BENCH / "corpus" / "must_refuse.json").read_text())["cases"]
        text = json.dumps(cases)
        for needle in ('"ton"', '"gal"', '"oz"', '"pt"', '"cal"', '"Btu"', '"hp"', '"mile"', '"ppm"', '"Nm3"', "wt%"):
            self.assertIn(needle, text)
        self.assertTrue(any(c["op"] == "add" for c in cases))

    def test_independent_cases_cite_a_source_and_are_oracle_checked(self):
        cases = json.loads((BENCH / "corpus" / "must_convert.json").read_text())["cases"]
        ind = [c for c in cases if c["origin"] == "independent"]
        self.assertGreaterEqual(len(ind), 25)
        for c in ind:
            self.assertRegex(c["source"], r"NIST|BIPM|Textbook|UCUM|mathjs|pint")
            self.assertTrue(c["oracle_checked"])

    def test_every_refuse_case_has_a_group(self):
        cases = json.loads((BENCH / "corpus" / "must_refuse.json").read_text())["cases"]
        for c in cases:
            self.assertEqual(c["group"], "ambiguous_name" if c["category"] == "ambiguous_name" else "other")
        self.assertTrue({c["group"] for c in cases} == {"ambiguous_name", "other"})

    def test_group_split_counts_accepted_ambiguous_names_apart(self):
        s = {"cases": {"must_refuse": [
            {"id": "a", "category": "ambiguous_name", "outcome": "silently_accepted"},
            {"id": "b", "category": "kind", "outcome": "silently_accepted"},
            {"id": "c", "category": "missing_context", "group": "other", "outcome": "refused"}]}}
        g = report.group_counts(s)
        self.assertEqual(g["ambiguous_name"]["silently_accepted"], 1)
        self.assertEqual(g["other"]["silently_accepted"], 1)
        self.assertEqual(g["other"]["refused"], 1)


class CheckVersions(unittest.TestCase):
    def write_result(self, d, name, version, language):
        (Path(d) / f"{name}-{version}.json").write_text(json.dumps({
            "kind": "library", "library": {"name": name, "version": version}, "runtime": {"language": language}}))

    def test_committed_results_match_the_lockfiles(self):
        self.assertEqual(check_versions.check_results(BENCH / "results"), [])

    def test_a_result_with_another_version_than_the_lockfile_is_reported(self):
        with tempfile.TemporaryDirectory() as d:
            self.write_result(d, "pint", "0.1", "python")
            self.write_result(d, "mathjs", "13.2.0", "javascript")
            problems = check_versions.check_results(d)
            self.assertEqual(len(problems), 1)
            self.assertIn("pint", problems[0])

    def test_pin_parsing_ignores_hashes_and_normalises_names(self):
        with tempfile.TemporaryDirectory() as d:
            f = Path(d) / "lock.txt"
            f.write_text("# c\nTyping_Extensions==4.16.0 \\\n    --hash=sha256:ab\npint==0.26.2 ; python_version>'3'\n")
            self.assertEqual(check_versions.python_pins(f), {"typing-extensions": "4.16.0", "pint": "0.26.2"})

    def test_empty_results_directory_is_a_problem(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertTrue(check_versions.check_results(d))


if __name__ == "__main__":
    unittest.main()
