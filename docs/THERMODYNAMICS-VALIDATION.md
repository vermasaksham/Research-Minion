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

## Independent comparison with the supplied table

`scripts/extract-binnewies-validation.py` extracts nine selected rows from
`Binnewies.Tabelle1.XLS` into
`tests/fixtures/binnewies-validation.json`. The fixture records the exact
`Tabelle1` row for each species and is used by
`tests/thermo_binnewies_validation.rs`. This is a second spreadsheet input,
independent of the chlorine calculation workbook; it is **not** a comparison
with the original Knacke PDF. The test converts table H from kJ/mol to J/mol
and the table heat-capacity coefficients to the engine's
`Cp = a + bT + c/T² + dT²` units (`b × 10⁻³`, `c × 10⁶`, `d × 10⁻⁶`). It
integrates from the table's 298 K H/S anchor. Blank coefficients mean zero.

| Reaction / table rows | T (K) | Table ΔG° (J/mol) | Chlorine workbook ΔG° (J/mol) | Table − workbook (J/mol) |
| --- | ---: | ---: | ---: | ---: |
| 2/3 SbCl3(g) → 2/3 Sb(s) + Cl2(g); rows 2188, 4966, 1954 | 900 | 184110.248 | 184123.284 | -13.036 |
| SeCl2(g) → Se(s) + Cl2(g); rows 2051, 4999, 1954 | 350 | 44079.374 | 44076.519 | +2.855 |
| TeCl2(s) → 1/2 Te2(g) + Cl2(g); rows 2068, 5166, 1954 | 298 | 212945.600 | 212949.079 | -3.479 |
| BiCl3(g) → BiCl(g) + Cl2(g); rows 888, 880, 1954 | 650 | 213259.796 | 205086.330 | +8173.466 |

The first three differences are small relative to the source table's rounded
H/S/Cp values. The solid `TeCl2` table row is valid only at 298 K, which is
why that independent check uses 298 K instead of the workbook-only 400 K
check above. The bismuth difference is much larger and remains unresolved.
For reproducibility, run:

```bash
python scripts/extract-binnewies-validation.py Binnewies.Tabelle1.XLS tests/fixtures/binnewies-validation.json
cargo test --test thermo_binnewies_validation
```

## Discrepancies requiring review

- `Red-SbCl3!AA44` at 923 K caches **218858.2646 J/mol**, while that row's
  displayed species Gibbs values imply **183209.4765 J/mol** for the liquid-Sb
  reaction. Its formula `=(2/3)*T44+Z44-(2/3)*Q44` still refers to `T44`
  (solid Sb), which is blank at this temperature. Excel treats that blank
  reference as zero, reproducing the cached result. Substituting `W44` (liquid
  Sb) gives the value implied by the row. The cached result and its derived
  pressure and E° are therefore inconsistent with the liquid-Sb reaction.
- `Red-BiCl3` anchors `BiCl3(g)` at 298 K with ΔfH° **-292270.554 J/mol**
  and S° **303.287 J/(mol K)**. `Binnewies.Tabelle1.XLS`, row 888, lists
  **-265300 J/mol** and **357.4 J/(mol K)** for the matching gas at 298 K.
  Differences are **-26970.554 J/mol** and **-54.113 J/(mol K)**. The imported
  candidate carries a prominent discrepancy warning. The workbook's
  `Red-BiCl3!I23` back-integration formula references `$C$13:$C$16`, the
  liquid `BiCl3` Cp column, while the gas coefficients are in `$D$13:$D$16`.
  This is a likely contributor, but the calculation workbook has not been
  repaired or fully re-audited. Matching its cached result at 650 K does not
  resolve this source disagreement.
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
