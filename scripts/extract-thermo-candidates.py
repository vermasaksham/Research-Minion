#!/usr/bin/env python3
"""Extract reviewable candidates from the user's chlorine calculation workbook.

Requires openpyxl. The output is spreadsheet-derived, never labelled Knacke-verified.
Formula caches must be present. No PDF or OCR is trusted by this script.
"""
import argparse
import json
import re

import openpyxl
from openpyxl.utils import get_column_letter


def audit_with_binnewies(sheet, candidate):
    """Compare same-temperature H/S anchors; never substitute one workbook for another."""
    import xlrd

    formula, phase = candidate["formula"], candidate["phase"]
    matches = []
    for row in range(2, sheet.nrows):
        if str(sheet.cell_value(row, 1)) != formula or str(sheet.cell_value(row, 3)) != phase:
            continue
        h_text, s_text = str(sheet.cell_value(row, 4)), str(sheet.cell_value(row, 7))
        h = re.match(r"^([+-]?\d+(?:\.\d+)?)", h_text)
        entropy = re.match(r"^([+-]?\d+(?:\.\d+)?)", s_text)
        if not h or not entropy:
            continue
        h_ref = re.search(r"\((\d+)\)", h_text)
        s_ref = re.search(r"\((\d+)\)", s_text)
        h_ref = int(h_ref.group(1)) if h_ref else 298
        s_ref = int(s_ref.group(1)) if s_ref else 298
        if h_ref == s_ref == candidate["reference_k"]:
            matches.append((row + 1, float(h.group(1)) * 1000, float(entropy.group(1))))
    if not matches:
        return "No same-temperature H/S row in Binnewies.Tabelle1.XLS; compare manually."
    row, h, entropy = min(matches, key=lambda record: abs(record[1] - candidate["enthalpy_j_mol"]))
    delta_h = candidate["enthalpy_j_mol"] - h
    delta_s = candidate["entropy_j_mol_k"] - entropy
    if abs(delta_h) > 100 or abs(delta_s) > 0.15:
        return (
            f"DISCREPANCY versus Binnewies.Tabelle1.XLS row {row}: "
            f"ΔH={delta_h:+.2f} J/mol, ΔS={delta_s:+.3f} J/(mol K)."
        )
    return None


def extract(path, binnewies_path=None):
    workbook = openpyxl.load_workbook(path, read_only=True, data_only=True)
    binnewies_sheet = None
    if binnewies_path:
        import xlrd
        binnewies_sheet = xlrd.open_workbook(binnewies_path).sheet_by_index(0)
    sources = []
    for sheet in workbook:
        entries = []
        for column in range(2, 11):
            marker = sheet.cell(12, column)
            match = re.search(
                r"([A-Za-z][A-Za-z0-9]*)\(([slg])\)\s*(\d+)\s*-\s*(\d+)\s*K",
                str(marker.value),
            )
            if not match:
                continue
            formula, phase, minimum, maximum = match.groups()
            minimum, maximum = int(minimum), int(maximum)
            result_column = 3 + (column - 2) * 3
            anchor = None
            for row in range(23, sheet.max_row + 1):
                temperature = sheet.cell(row, 1).value
                enthalpy = sheet.cell(row, result_column).value
                entropy = sheet.cell(row, result_column + 1).value
                if (
                    isinstance(temperature, (int, float))
                    and minimum <= temperature <= maximum
                    and isinstance(enthalpy, (int, float))
                    and isinstance(entropy, (int, float))
                ):
                    anchor = (row, temperature, enthalpy, entropy)
                    break
            if anchor is None:
                continue
            coefficients = [sheet.cell(row, column).value for row in range(13, 17)]
            if not all(isinstance(value, (int, float)) for value in coefficients):
                continue
            row, temperature, enthalpy, entropy = anchor
            h_cell = f"{get_column_letter(result_column)}{row}"
            s_cell = f"{get_column_letter(result_column + 1)}{row}"
            candidate = dict(
                    formula=formula,
                    phase=phase,
                    min_k=minimum,
                    max_k=maximum,
                    reference_k=temperature,
                    enthalpy_j_mol=enthalpy,
                    entropy_j_mol_k=entropy,
                    cp_coefficients=coefficients,
                    source_locator=f"{sheet.title}!{marker.coordinate}, {h_cell}, {s_cell}; Cp {get_column_letter(column)}13:{get_column_letter(column)}16",
                )
            if binnewies_sheet is not None:
                candidate["warning"] = audit_with_binnewies(binnewies_sheet, candidate)
            entries.append(candidate)
        sources.append(dict(name=sheet.title, entries=entries))
    return sources


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("workbook", help="the user's .xlsx chlorine calculation workbook")
    parser.add_argument("output", help="reviewable JSON output")
    parser.add_argument("--binnewies-xls", help="optional source table for H/S discrepancy audit")
    args = parser.parse_args()
    with open(args.output, "w", encoding="utf-8") as handle:
        json.dump(extract(args.workbook, args.binnewies_xls), handle, indent=2, ensure_ascii=False)
        handle.write("\n")
