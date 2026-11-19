#!/usr/bin/env python3
# Copyright (c) 2026 Moonbase 2090 LLC
# SPDX-License-Identifier: MIT
"""Independent policy fixtures for the widget instruction gate."""
import argparse
from fractions import Fraction
import importlib.util
from pathlib import Path
import sys
import unittest

spec = importlib.util.spec_from_file_location("widget_instructions", Path(__file__).with_name("check-widget-instructions.py"))
gate = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = gate
spec.loader.exec_module(gate)


class InstructionGateTests(unittest.TestCase):
    def test_clock_noise_never_changes_the_instruction_decision(self):
        for elapsed in [0.000001, 0.64, 0.80, 1.01, 1.07, 999.0, float("nan"), float("inf")]:
            frame = gate.Frame("effects 80x24", gate.Sample(103, elapsed), gate.Sample(100, 1.0))
            self.assertEqual(gate.regressions([frame], Fraction(3)), [])

    def test_extra_work_fails_even_when_its_clock_looks_faster(self):
        frame = gate.Frame("effects 80x24", gate.Sample(104, 0.1), gate.Sample(100, 1.0))
        self.assertEqual(gate.regressions([frame], Fraction(3)),
                         ["effects 80x24 instructions 104 exceed main 100 by more than 3%"])

    def test_large_counts_do_not_round_away_a_regression(self):
        frame = gate.Frame("x", gate.Sample(10300000000000000001, 0.1), gate.Sample(10000000000000000000, 1.0))
        self.assertEqual(gate.regressions([frame], Fraction(3)),
                         ["x instructions 10300000000000000001 exceed main 10000000000000000000 by more than 3%"])

    def test_a_new_case_reports_without_an_invented_baseline(self):
        self.assertEqual(gate.regressions([gate.Frame("new", gate.Sample(100, 1.0), None)], Fraction(3)), [])

    def test_callgrind_declares_the_event_order(self):
        self.assertEqual(gate.parse_count("events: Dr Ir Dw\nsummary: 7 12345 9\n"), 12345)

    def test_missing_or_invalid_counts_cannot_disable_the_guard(self):
        for text, message in [
            ("", "Callgrind output has no Ir event"),
            ("events: Dr\nsummary: 1\n", "Callgrind output has no Ir event"),
            ("events: Ir\n", "Callgrind summary does not match its events"),
            ("events: Ir Dr\nsummary: 1\n", "Callgrind summary does not match its events"),
            ("events: Ir\nsummary: 0\n", "Callgrind Ir must be a positive integer"),
            ("events: Ir\nsummary: NaN\n", "Callgrind Ir must be a positive integer"),
        ]:
            with self.assertRaises(ValueError) as caught:
                gate.parse_count(text)
            self.assertEqual(str(caught.exception), message)

    def test_only_finite_nonnegative_limits_are_accepted(self):
        self.assertEqual(gate.percentage("3"), Fraction(3))
        self.assertEqual(gate.percentage("0"), Fraction(0))
        for bad in ["NaN", "inf", "-1", "abc"]:
            with self.assertRaises(argparse.ArgumentTypeError):
                gate.percentage(bad)


if __name__ == "__main__":
    unittest.main()
