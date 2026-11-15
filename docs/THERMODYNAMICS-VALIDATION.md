# Thermodynamic spreadsheet validation and limits

The user supplied `Binnewies.Tabelle1.XLS` and
`Partial_pressure_Chlorine_Sb-Bi-Te-Se.xlsx`. The first is a thermochemical
table. The second contains four chlorine calculation worksheets with cached
results. The 1991 Knacke PDF exceeded this workspace's 32 MiB transfer limit;
the app does **not** label spreadsheet values as verified Knacke records.

`scripts/extract-thermo-candidates.py` extracts 28 phase-specific candidates
from the four calculation sheets. It records each phase range, H/S anchor,
polynomial heat-capacity coefficients, and cell locator. The script compares
same-temperature H/S anchors with the supplied Binnewies table. In the app,
**Thermodynamic Databases → Import supplied worksheet candidates** creates
four local sources and leaves every record **unverified**. Reimporting does not
overwrite reviewed records. A researcher must review the source cells and
mark each record verified before calculations can use it.

The engine integrates `Cp(T) = a + bT + c/T² + dT²` and uses signed rational
reaction coefficients. `tests/thermo_workbook_validation.rs` compares four
independent Rust calculations with cached worksheet values. The test inputs
come from the extracted worksheet cells, and the expected outputs come from
separate cached calculation cells. All four cases pass at these tolerances:
ΔH and ΔG 0.05 J/mol; ΔS 0.0001 J/(mol K); log₁₀ pressure 0.00001; E°
0.00001 V. The pressure comparison sets every other ideal-gas activity to 1.

| Worksheet | Reaction | T (K) | ΔG° (J/mol) | log₁₀(p(Cl₂)/bar) | E° (V, z=2) |
| --- | --- | ---: | ---: | ---: | ---: |
| Red-SbCl3 | 2/3 SbCl3(g) → 2/3 Sb(s) + Cl2(g) | 900 | 184123.2839 | -10.686029 | -0.954155 |
| Red-SeCl2 | SeCl2(g) → Se(s) + Cl2(g) | 350 | 44076.5194 | -6.577932 | -0.228411 |
| Red-TeCl2 | TeCl2(s) → 1/2 Te2(g) + Cl2(g) | 400 | 193779.5295 | -25.304516 | -1.004195 |
| Red-BiCl3 | BiCl3(g) → BiCl(g) + Cl2(g) | 650 | 205086.3295 | -16.480618 | -1.062789 |

These comparisons validate the implementation of the worksheet equations at
the selected points. They do not independently validate the input values or
the physical choice of phases.

## Discrepancies requiring review

- `Red-SbCl3!AA44` at 923 K caches **218858.2646 J/mol**, while that row's
  displayed species Gibbs values imply **183209.4765 J/mol** for the liquid-Sb
  reaction. The formula in `AA44` still refers to `T44` (solid Sb), which is
  blank at this temperature. The 923 K cached result is not a valid
  independent check of the liquid-Sb calculation.
- `Red-BiCl3` anchors `BiCl3(g)` at 298 K with ΔfH° **-292270.554 J/mol**
  and S° **303.287 J/(mol K)**. `Binnewies.Tabelle1.XLS`, row 888, lists
  **-265300 J/mol** and **357.4 J/(mol K)** for the matching gas at 298 K.
  Differences are **-26970.554 J/mol** and **-54.113 J/(mol K)**. The imported
  candidate carries a prominent discrepancy warning. Matching the calculation
  worksheet at 650 K does not resolve this source disagreement.
- `SbCl5(g)` has no same-temperature H/S anchor in the two files: the
  calculation worksheet anchors it at 413 K, while the matching table row is
  at 298 K. The candidate is flagged for manual comparison.

The calculator stops at missing, unverified, overlapping or out-of-range
records. Automatic phase selection does not guess a stable phase from
overlapping validity ranges. Saved results contain record IDs, revisions,
source locators and the input snapshot. The Excel export copies the user's
`.xlsx` template and adds a `CrystalForge Results` worksheet. It leaves all
existing sheets, formulas and formatting in place; it does not rewrite
template formulas or claim to repair the discrepancies above. CSV export is
also available.
