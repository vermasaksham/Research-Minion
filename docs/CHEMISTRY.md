# Material input conventions

The periodic-table selector bundles atomic numbers, symbols, and English names
for all 118 named elements. Naming follows the usual IUPAC conventions, including
Aluminium, Sulfur, and Caesium; common alternatives Aluminum, Sulphur, and Cesium
are accepted as name suggestions. No atomic weights, thermochemical data, or
structure/reference measurements are included. Naming reference:
[IUPAC periodic table](https://iupac.org/what-we-do/periodic-table-of-elements/).

The selector uses a conventional 18-column layout with the 57–71 and 89–103
series detached below. Element buttons show the atomic number and symbol; hover
reveals the name. The precursor and obtained-material selections are independent.

## Manual descriptions and suggestions

Each material stores two separate things: the original text and atomic numbers
explicitly selected by the researcher. Typing does not silently change the confirmed
element set. Click individual suggestions, Add suggested, or periodic-table buttons.
Removing a chip does not delete the typed description. Unknown names remain valid
descriptions and can be saved without selecting elements.

Formula matching recognises case-sensitive element symbols and balanced brackets,
Unicode numeric subscripts, counts, separators and hydrate dots. Counts are ignored
when collecting element identities; their stoichiometric correctness is not checked.
For example, `Sb₂Se₃` suggests Sb/Se, `CuSO4·5H2O` suggests Cu/S/O/H, and `Co`
differs from `CO`. This helper is not a formula validator or equation balancer.

Name matching recognises the 118 element names and incomplete final element-name
prefixes. A bounded vocabulary covers common parts such as oxide, selenide, sulfide,
chloride, sulfate, nitrate, hydroxide and hydrate, including familiar numeric
prefixes such as di-/tri-. A few common names such as water, ammonia, methane,
ethanol and acetone are included. It does not resolve every compound name, registry
identifier, trade name, solvent mixture, structure, or oxidation state. There are
no web lookups, language models, or predictions of what a reaction will produce.

## Endothermic and exothermic

Thermal character is an optional researcher assignment: Not specified, Endothermic,
or Exothermic. Endothermic describes heat absorption for the specified reaction;
exothermic describes heat release. Which reaction/direction is being described
should be documented in the experiment log. A source/sink temperature difference
is not a reaction enthalpy and never determines this field automatically.

The transport agent is optional free text, independent of precursor/product element
sets. A blank field is displayed as Not recorded, rather than asserting no agent
was used. Raw material and agent descriptions are retained as entered; selected
atomic numbers are checked against the local element catalog before saving.
