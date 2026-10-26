# Peak Explorer 0.7: independent EDX and XRD references

Open **Scientific Utilities → Peak Explorer**. Everything runs locally in Rust.
The default **Side by side** view separates emitted photon energies in **keV**
from angle-dispersive diffraction positions in **degrees 2θ**. Marker heights are
schematic; there are no measured spectra or predicted intensities.

## Quick lab workflow

1. Select EDX elements and XRD phases independently. The Sb₂Se₃/Sb/Se presets
   change both selections; the Sb + Se + I shortcut changes only EDX elements.
2. Click a marker or table row to inspect its exact reference and source.
3. For EDX, choose a line family, energy window and electron-beam voltage.
   Pale dashed sticks mean the beam is at/below that shell's ionisation edge.
4. For XRD, choose Cu Kα₁, Co Kα₁ or a positive custom wavelength in Å. Use your
   instrument's configured wavelength when matching a measurement.
5. Toggle **Sb₂Se₃: mapped Pnma indices** to map the original Pbnm cell and hkl
   together. The underlying d-spacings and angles remain unchanged.
6. **Lab guide & sources** contains concise interpretation notes, citations,
   reported cell values, copied source CIFs, retrieval records and source hashes.
7. Expand **Save / export reference snapshot**, add an interpretation and choose
   an experiment/event date. Save a new analysis note, copy JSON or export JSON
   into the active profile's `exports` folder. Existing saved snapshots are not
   silently rewritten by later control changes.

## EDX data: seven elements, 46 selected markers

**Sb, Bi, Se, I, Cl, Br and S** use NIST SRD 128's **Direct experimental** column.
Each entry retains the transition, exact reported energy/uncertainty strings,
reported blend status, source row transitions, ionisation edge/uncertainty and
source URL. The Bi table specifies mass number A=209. Missing direct experimental
lines are omitted; adjacent theoretical or combined columns are not substituted.
This is a selected K/L reference, not an exhaustive catalog of every emission.

Reported blends are single shared markers: Se/Br Lα₁,₂, Sb/I Lβ₂,₁₅ and Cl Kβ.
They are not displayed as independently resolved experimental lines. The source
uncertainties describe the reference values, not your detector's energy resolution.

Example: Sb Lα₁ is **3604.756 ± 0.062 eV = 3.604756 keV**. A 12 kV beam has more
energy than the Se Kα₁ photon, but remains below the **12.65461 keV K edge**;
using emitted photon energy as the excitation threshold would give the wrong answer.
The beam must exceed the initial shell's edge for direct electron ionisation.
Being above an edge does not guarantee useful counts or model detector response,
chemical shifts, overvoltage, matrix effects or peak overlap.

Source: Deslattes et al., *X-Ray Transition Energies Database*, NIST Standard
Reference Database 128, DOI **10.18434/T4859Z**.

## XRD data: three structures, 48 selected reflections

| Phase | Source cell a, b, c (Å) | Crystal setting | Reference |
| --- | --- | --- | --- |
| Sb₂Se₃ | 11.62, 11.77, 3.962 | Orthorhombic Pbnm, no. 62 | COD 9007437 |
| Sb | 4.3084(2), 4.3084(2), 11.274(6) | R−3m, no. 166; hexagonal axes | COD 5000214 |
| Trigonal Se | 4.3662, 4.3662, 4.9536 | P3₁21, no. 152; hexagonal axes | COD 9012501 |

Sb₂Se₃ has α=β=γ=90°. The Sb/Se conventional cells have α=β=90°, γ=120°.
The reciprocal metric handles those non-orthogonal axes correctly. Calculated
spacings are independently checked against Gemmi using the source CIFs. Each
of the 48 curated hkl positions is checked against that CIF's symmetry operations.
This does **not** generate a complete powder pattern or predict structure-factor
intensities. Displayed decimal places do not imply matching experimental accuracy.

Citations retained with the source files:

- Tideswell, Kruse & McCullough (1957), *Acta Crystallographica* 10, 99–102.
  DOI: **10.1107/S0365110X57000298**.
- Barrett, Cucka & Haefner (1963), *Acta Crystallographica* 16, 451–453.
  DOI: **10.1107/S0365110X63001262**.
- Cherin & Unger (1967), *Inorganic Chemistry* 6, 1589–1591.
  DOI: **10.1021/ic50054a037**.

The CIFs were recovered from the linked original Peak Explorer webpage. Their
SHA-256 values match its data file; exact retained copies and canonical COD links
are bundled. The catalog version is **2026-10-26.1**.

**Iodine-containing XRD phase references, including SbSeI and SbI₃, are not
bundled.** Direct COD access remained blocked during this build, so additional
structures could not be verified. Iodine EDX lines cannot distinguish residual
transport agent, SbI₃ and SbSeI. Use appropriate verified structures and additional
evidence before assigning those phases.

## Wavelength, settings and the comparison

First-order Bragg law uses `2θ = 2 asin(λ/(2d))`. For λ>2d there is no solution;
the reference row remains and no marker is plotted. d depends on the cell and
hkl; 2θ additionally depends on the incident wavelength.

The NIST direct experimental Cu/Co Kα₁ energies are **8047.8227 ± 0.0026 eV** and
**6930.3780 ± 0.0039 eV**. Wavelengths are calculated in vacuum from
`λ = hc/E`, with `hc = 12398.419843320026 eV Å` from exact SI definitions.
These can differ slightly from conventional instrument wavelength tables.

COD 9007437 is Pbnm. For Pnma the tool maps `(a',b',c')=(b,c,a)` and
`(h',k',l')=(k,l,h)`. Thus `(240)` becomes `(402)` with unchanged d and 2θ.
For this reference, d≈**2.6250398942 Å**; the Cu and Co positions are approximately
**34.12825°** and **39.84588°**. Sb Lα₁ remains **3.604756 keV**.

Converting an emitted photon energy into a wavelength does not select crystal
planes. A diffraction angle also requires d and a geometry. In monochromatic
XRD, different reflections scatter essentially the same incident photon energy
at different angles. An Sb+Se mixture can share the elemental EDX line families
of Sb₂Se₃: EDX alone does not establish that crystalline phase.

## Local snapshots and manual calculations

Exports contain explicit quantities/units, all selected references (including
those outside the plot window), cells, original/displayed indices, radiation,
beam, source hashes and interpretation. JSON exports use unique filenames inside
the owning profile. Saved log snapshots are editable notes, not immutable audits.
Tool drafts remain in memory under their profile until the app closes; save to
retain a durable snapshot. Source links open only when clicked; no analysis is
uploaded and no online service is needed to use the tool.

**Manual XRD** retains the earlier orthorhombic calculator and forward/inverse
Bragg calculators. Manual axes/indices are used as entered; setting names in its
free-text field do not transform them. It does not apply symmetry filtering or
silently substitute the bundled catalog. Its calculation ID remains
`orthorhombic-bragg/1`; the reference explorer uses
`edx-direct-and-cif-metric-bragg/1`.

## Reproduce the data validation

Python is only needed to audit/rebuild the reference catalog, not to run the app:

```sh
python -m pip install -r scripts/peak-validation-requirements.txt
python scripts/build-peak-catalog.py --check
```

The pinned Gemmi 0.7.5 validator works entirely from retained files, checks source
hashes, direct experimental values/blends, all 48 metrics/systematic absences,
and both radiation lines, and compares the regenerated catalog exactly. The ZIP
includes `data/peak` and the validation script. Rust tests independently compare
its parsed values with the raw NIST columns and test metric/setting/units behavior.
