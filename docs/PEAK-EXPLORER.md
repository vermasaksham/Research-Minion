# Peak Explorer 0.4: initial offline XRD tool

Open **Scientific Utilities → Peak Explorer**. This increment calculates positions from a
**user-entered orthorhombic cell** and explicit reflection indices. It works fully
offline and has no dependencies on Python, external websites or reference services.
It is a new, limited implementation; the original private Sb₂Se₃ Peak Explorer
source and datasets have not been recovered or ported.

## Inputs and conventions

- a, b, c in Å, finite and greater than zero. All three interaxial angles are 90°.
  The calculator is unsuitable for monoclinic, triclinic, hexagonal or rhombohedral
  conventional cells. No crystal-system conversion occurs.
- Wavelength λ in Å, finite and greater than zero. The initial value 1.5406 Å is
  editable; record the wavelength convention and source appropriate to your work.
  No assumption about an unresolved Kα doublet is applied.
- One signed integer `h k l` per line, with spaces or commas. Up to 500 explicit
  reflections; duplicates, malformed lines and 0 0 0 are rejected. Entered order
  is retained. No automatic symmetry-allowed reflection list is generated.
- Optional material label, axis/structure-setting description, source/provenance
  and interpretation. These are researcher-supplied descriptions, not verified
  structural data. Axes and indices are used exactly as entered; labelling a cell
  Pbnm or Pnma does not transform its axes or reflections.

For a synthetic check, enter a=3, b=4, c=12 Å, λ=2 Å and reflection 1 0 0.
Then d=3 Å and 2θ=38.942441°. These numbers illustrate the formulas; this is not
a reference unit cell for a material.

## Calculations

For orthorhombic axes:

```text
1/d² = h²/a² + k²/b² + l²/c²
```

First-order Bragg law (n=1):

```text
2θ = 2 asin(λ / (2d))
d  = λ / (2 sin(θ))
```

Angles are displayed in degrees. For λ>2d there is no first-order Bragg solution;
the row remains visible and no marker is plotted. λ=2d gives 2θ=180°. The inverse
calculator accepts 0<2θ≤180°. Invalid inputs and unsupported floating-point ranges
return errors instead of NaN results. `orthorhombic-bragg/1` identifies this
calculation convention. Tests use independent analytic values, not missing
material reference datasets.

Marker heights are equal and have no physical meaning. Hover over a marker to
see nearby reflections. The table displays rounded values; saved reports retain
round-trip precision decimal values for numeric inputs and results.

## Saving

Choose an existing experiment, supply an analysis event date (`YYYY-MM-DD`), and
click **Save analysis to log**. This creates a new ordinary log entry containing
the cell and wavelength, explicit indices, source/setting descriptions, all
calculated results, app/calculation versions, calculation UTC time, limitations,
and your interpretation. Open that experiment's bar to read it. Editing inputs
later does not rewrite a saved snapshot; clicking Save again makes another entry.
Saved notes remain editable and therefore are not immutable audit records.

Only saved analysis entries are durable. Input fields are retained during the
current app process, without crash recovery or automatic persistence of tool drafts.

## Pending original-tool integration

The 15 NIST-attributed Sb/Se EDX markers, the original 48 selected reflections and
three CIFs, explicit Pbnm/Pnma conversion, and JSON export are not included.
Original-source integration requires the retained files and their checks/provenance.
No numeric reference catalog has been recreated from a prose description.

This build does not filter systematic absences or predict intensities, phase
fractions, or phase identity. XRD positions in 2θ degrees are distinct from EDX
characteristic energies in keV. Do not interpret a position match alone as evidence
of a particular phase.
