#!/usr/bin/env python3
"""Extract a small, traceable validation fixture from the supplied Binnewies XLS.

Usage: python scripts/extract-binnewies-validation.py Binnewies.Tabelle1.XLS \
       tests/fixtures/binnewies-validation.json

Requires xlrd. This reads the spreadsheet table, not the unavailable Knacke PDF.
Only the nine rows needed for four chlorine reaction checks are reproduced.
"""
import argparse
import json
import re

import xlrd


ROWS = {
    880: ("BiCl", "g"),
    888: ("BiCl3", "g"),
    1954: ("Cl2", "g"),
    2051: ("SeCl2", "g"),
    2068: ("TeCl2", "s"),
    2188: ("SbCl3", "g"),
    4966: ("Sb", "s"),
    4999: ("Se", "s"),
    5166: ("Te2", "g"),
}


def extract(path):
    book = xlrd.open_workbook(path)
    sheet = book.sheet_by_name("Tabelle1")
    entries = []
    for row_number, (formula, phase) in ROWS.items():
        cells = sheet.row_values(row_number - 1)
        assert cells[1] == formula and cells[3] == phase, (
            f"Unexpected identity at Tabelle1 row {row_number}: {cells[1]}, {cells[3]}"
        )
        assert isinstance(cells[4], (int, float)) and isinstance(cells[7], (int, float))
        assert not cells[5] and not cells[8], f"Temperature-annotated anchor at {row_number}"
        validity = str(int(cells[14])) if isinstance(cells[14], (int, float)) else str(cells[14])
        bounds = re.fullmatch(r"(\d+)(?:\s*-\s*(\d+))?", validity)
        assert bounds, f"Unknown validity range at {row_number}: {validity!r}"
        minimum = float(bounds.group(1))
        maximum = float(bounds.group(2) or bounds.group(1))
        assert minimum == 298, f"Unexpected reference temperature at {row_number}"
        coefficients = []
        for column, factor in zip(range(10, 14), (1, 1e-3, 1e6, 1e-6)):
            raw = cells[column]
            assert raw == "" or isinstance(raw, (int, float)), (
                f"Unexpected Cp coefficient at {row_number}, column {column + 1}"
            )
            coefficients.append(round((raw or 0) * factor, 12))
        entries.append({
            "formula": formula,
            "phase": phase,
            "min_k": minimum,
            "max_k": maximum,
            "reference_k": 298.0,
            "enthalpy_j_mol": cells[4] * 1000,
            "entropy_j_mol_k": cells[7],
            "cp_coefficients": coefficients,
            "source_locator": f"Binnewies.Tabelle1.XLS Tabelle1 row {row_number}",
        })
    return entries


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("xls", help="user-supplied Binnewies.Tabelle1.XLS")
    parser.add_argument("output", help="JSON fixture to write")
    args = parser.parse_args()
    with open(args.output, "w", encoding="utf-8") as output:
        json.dump(extract(args.xls), output, ensure_ascii=False, indent=2)
        output.write("\n")
