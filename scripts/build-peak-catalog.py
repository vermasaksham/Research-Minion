#!/usr/bin/env python3
"""Rebuild the offline reference catalog from retained sources; no network access.

Requires gemmi==0.7.5. Run from any directory. --check validates without writing.
"""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path

import gemmi

ROOT = Path(__file__).resolve().parents[1] / "data" / "peak"
ELEMENTS = [("Sb", "Antimony", 51), ("Bi", "Bismuth", 83),
            ("Se", "Selenium", 34), ("I", "Iodine", 53),
            ("Cl", "Chlorine", 17), ("Br", "Bromine", 35), ("S", "Sulfur", 16)]
TRANSITIONS = {"KL3": "Kα₁", "KL2": "Kα₂", "KM3": "Kβ₁", "KM2": "Kβ₃",
               "L3M5": "Lα₁", "L3M4": "Lα₂", "L2M4": "Lβ₁",
               "L3N4": "Lβ₁₅", "L3N5": "Lβ₂", "L2N4": "Lγ₁"}
BLENDS = {"L3M4,5": "Lα₁,₂ (blend)", "L3N4,5": "Lβ₂,₁₅ (blend)", "KM": "Kβ (blend)"}
HC = 12398.419843320026  # exact SI h*c/e, eV Å


def nist_rows(element):
    lines = (ROOT / "sources" / f"{element}-all.tsv").read_text(encoding="utf-8").splitlines()
    assert lines[4].split("\t")[5:7] == ["Direct (eV)", "Unc. (eV)"]
    rows = list(csv.reader(lines[5:], delimiter="\t"))
    assert all(len(r) == 13 and r[0] == element for r in rows)
    return {r[2]: r for r in rows}


def build():
    manifest = json.loads((ROOT / "source-manifest.json").read_text(encoding="utf-8"))
    for source in manifest:
        assert hashlib.sha256((ROOT / source["file"]).read_bytes()).hexdigest() == source["sha256"], source["file"]
    website = json.loads((ROOT / "sources" / "website-data.json").read_text(encoding="utf-8"))
    elements, lines = [], []
    for symbol, name, number in ELEMENTS:
        url = f"https://physics.nist.gov/cgi-bin/XrayTrans/search.pl?element={symbol}&trans=All&units=eV"
        elements.append(dict(symbol=symbol, name=name, atomic_number=number, source=url))
        rows = nist_rows(symbol)
        seen_blends = set()
        for transition, label in TRANSITIONS.items():
            r = rows.get(transition)
            if not r or not r[5]:  # Never fill missing measured data from theory.
                continue
            blend = r[11]
            source_transitions = [transition]
            if blend:
                if blend in seen_blends:
                    continue
                partners = [q for key, q in rows.items() if key in TRANSITIONS and q[11] == blend and q[5]]
                assert all(q[5:7] == r[5:7] for q in partners), (symbol, blend)
                source_transitions = [q[2] for q in partners]
                label = BLENDS[blend]
                seen_blends.add(blend)
            shell = "K" if transition.startswith("K") else transition[:2]
            edge = rows[f"{shell} edge"]
            assert edge[5] and edge[6], (symbol, shell, "No direct experimental edge")
            energy, uncertainty = float(r[5]), float(r[6])
            edge_energy, edge_uncertainty = float(edge[5]), float(edge[6])
            assert 0 < energy < edge_energy and 0 < uncertainty and 0 < edge_uncertainty
            transition_label = blend or transition
            lines.append(dict(
                id=f"{symbol}-{transition_label.replace(',', '')}", element=symbol,
                label=label, transition=transition_label, source_transitions=source_transitions,
                family=transition[0], energy_ev=energy, uncertainty_ev=uncertainty,
                reported_energy_ev=r[5], reported_uncertainty_ev=r[6],
                mass_number=r[1], blend=bool(blend), shell=shell,
                edge_ev=edge_energy, edge_uncertainty_ev=edge_uncertainty,
                edge_reported_ev=edge[5], edge_reported_uncertainty_ev=edge[6],
                measurement="NIST direct experimental", reference_codes=r[12],
                source=url, source_file=f"sources/{symbol}-all.tsv",
                note=("One shared experimental marker; these transitions are not separately resolved in this reference." if blend else "")))
    for old in website["edx"]:
        matches = [r for r in lines if r["element"] == old["element"] and r["transition"] == old["transition"]]
        assert len(matches) == 1, old["id"]
        assert matches[0]["energy_ev"] == old["energyEV"] and matches[0]["uncertainty_ev"] == old["uncertaintyEV"]
        assert matches[0]["edge_ev"] == old["edgeEV"]

    phases = []
    specs = website["phases"][:]
    additional = ROOT / "additional-phases.json"
    if additional.exists():
        specs.extend(json.loads(additional.read_text(encoding="utf-8")))
    checked_reflections = 0
    for p in specs:
        raw = (ROOT / "sources" / f"{p['cod']}.cif").read_text(encoding="utf-8")
        block = gemmi.cif.read_string(raw).sole_block()
        cell = gemmi.UnitCell(*(gemmi.cif.as_number(block.find_value("_cell_length_" + axis)) for axis in "abc"),
                             *(gemmi.cif.as_number(block.find_value("_cell_angle_" + angle)) for angle in ["alpha", "beta", "gamma"]))
        values = [cell.a, cell.b, cell.c]
        angles = [cell.alpha, cell.beta, cell.gamma]
        ops = list(block.find_values("_space_group_symop_operation_xyz")) or list(block.find_values("_symmetry_equiv_pos_as_xyz"))
        assert ops, p["cod"]
        group = gemmi.GroupOps([gemmi.Op(gemmi.cif.as_string(s)) for s in ops])
        reflections = []
        for r in p["reflections"]:
            assert not group.is_systematically_absent(r["hkl"]), (p["cod"], r["hkl"])
            d = cell.calculate_d(r["hkl"])
            if "d" in r:
                assert math.isclose(d, r["d"], rel_tol=1e-12, abs_tol=1e-12), (p["cod"], r["id"])
            reflections.append(dict(id=r["id"], hkl=r["hkl"], d_a=d))
            checked_reflections += 1
        if "cell" in p:
            assert values == p["cell"], (p["cod"], values, p["cell"])
        digest = hashlib.sha256(raw.encode()).hexdigest()
        assert digest == p["sha256"], p["cod"]
        phases.append(dict(id=p["id"], label=p["label"], name=p["name"],
                           cod=p["cod"], space_group=p["spaceGroup"], system=p["system"],
                           lengths_a=values, angles_deg=angles,
                           cell_reported=[gemmi.cif.as_string(block.find_value("_cell_length_" + axis)) for axis in "abc"],
                           citation=p["citation"], doi=p["doi"], source=p["source"],
                           source_file=f"sources/{p['cod']}.cif", sha256=digest, cif_text=raw,
                           reflections=reflections))
    radiation = []
    for element, rid in [("Cu", "cu"), ("Co", "co")]:
        r = nist_rows(element)["KL3"]
        assert r[5] and r[6]
        energy = float(r[5])
        old = next(x for x in website["radiation"] if x["id"] == rid)
        assert energy == old["energyEV"]
        radiation.append(dict(id=rid, label=f"{element} Kα₁", energy_ev=energy,
                              uncertainty_ev=float(r[6]), wavelength_a=HC / energy,
                              source=old["source"], reported_energy_ev=r[5], reported_uncertainty_ev=r[6]))
    print(f"Validated {len(lines)} direct experimental EDX markers, {len(phases)} CIFs, {checked_reflections} allowed metric spacings, 2 radiation lines and all retained source hashes.")
    return dict(schema=1, version="2026-10-26.1", checked="2026-10-26", hc_ev_a=HC,
                elements=elements, edx=lines, phases=phases, radiation=radiation, sources=manifest)


if __name__ == "__main__":
    args = argparse.ArgumentParser()
    args.add_argument("--check", action="store_true")
    opts = args.parse_args()
    result = json.dumps(build(), indent=2, ensure_ascii=False) + "\n"
    path = ROOT / "catalog.json"
    if opts.check:
        assert path.read_text(encoding="utf-8") == result, "Catalog does not match retained source data"
        print("Bundled catalog matches regenerated values exactly.")
    else:
        path.write_text(result, encoding="utf-8", newline="\n")
