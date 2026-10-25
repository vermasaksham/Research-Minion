//! Offline element suggestions, not a reaction predictor or compound database.
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
pub struct Element {
    pub number: u8,
    pub symbol: &'static str,
    pub name: &'static str,
}

macro_rules! el {
    ($number:literal, $symbol:literal, $name:literal) => {
        Element {
            number: $number,
            symbol: $symbol,
            name: $name,
        }
    };
}

/// IUPAC element symbols/names, atomic numbers 1–118. No weights or other
/// reference measurements are bundled by this element selector.
pub const ELEMENTS: [Element; 118] = [
    el!(1, "H", "Hydrogen"),
    el!(2, "He", "Helium"),
    el!(3, "Li", "Lithium"),
    el!(4, "Be", "Beryllium"),
    el!(5, "B", "Boron"),
    el!(6, "C", "Carbon"),
    el!(7, "N", "Nitrogen"),
    el!(8, "O", "Oxygen"),
    el!(9, "F", "Fluorine"),
    el!(10, "Ne", "Neon"),
    el!(11, "Na", "Sodium"),
    el!(12, "Mg", "Magnesium"),
    el!(13, "Al", "Aluminium"),
    el!(14, "Si", "Silicon"),
    el!(15, "P", "Phosphorus"),
    el!(16, "S", "Sulfur"),
    el!(17, "Cl", "Chlorine"),
    el!(18, "Ar", "Argon"),
    el!(19, "K", "Potassium"),
    el!(20, "Ca", "Calcium"),
    el!(21, "Sc", "Scandium"),
    el!(22, "Ti", "Titanium"),
    el!(23, "V", "Vanadium"),
    el!(24, "Cr", "Chromium"),
    el!(25, "Mn", "Manganese"),
    el!(26, "Fe", "Iron"),
    el!(27, "Co", "Cobalt"),
    el!(28, "Ni", "Nickel"),
    el!(29, "Cu", "Copper"),
    el!(30, "Zn", "Zinc"),
    el!(31, "Ga", "Gallium"),
    el!(32, "Ge", "Germanium"),
    el!(33, "As", "Arsenic"),
    el!(34, "Se", "Selenium"),
    el!(35, "Br", "Bromine"),
    el!(36, "Kr", "Krypton"),
    el!(37, "Rb", "Rubidium"),
    el!(38, "Sr", "Strontium"),
    el!(39, "Y", "Yttrium"),
    el!(40, "Zr", "Zirconium"),
    el!(41, "Nb", "Niobium"),
    el!(42, "Mo", "Molybdenum"),
    el!(43, "Tc", "Technetium"),
    el!(44, "Ru", "Ruthenium"),
    el!(45, "Rh", "Rhodium"),
    el!(46, "Pd", "Palladium"),
    el!(47, "Ag", "Silver"),
    el!(48, "Cd", "Cadmium"),
    el!(49, "In", "Indium"),
    el!(50, "Sn", "Tin"),
    el!(51, "Sb", "Antimony"),
    el!(52, "Te", "Tellurium"),
    el!(53, "I", "Iodine"),
    el!(54, "Xe", "Xenon"),
    el!(55, "Cs", "Caesium"),
    el!(56, "Ba", "Barium"),
    el!(57, "La", "Lanthanum"),
    el!(58, "Ce", "Cerium"),
    el!(59, "Pr", "Praseodymium"),
    el!(60, "Nd", "Neodymium"),
    el!(61, "Pm", "Promethium"),
    el!(62, "Sm", "Samarium"),
    el!(63, "Eu", "Europium"),
    el!(64, "Gd", "Gadolinium"),
    el!(65, "Tb", "Terbium"),
    el!(66, "Dy", "Dysprosium"),
    el!(67, "Ho", "Holmium"),
    el!(68, "Er", "Erbium"),
    el!(69, "Tm", "Thulium"),
    el!(70, "Yb", "Ytterbium"),
    el!(71, "Lu", "Lutetium"),
    el!(72, "Hf", "Hafnium"),
    el!(73, "Ta", "Tantalum"),
    el!(74, "W", "Tungsten"),
    el!(75, "Re", "Rhenium"),
    el!(76, "Os", "Osmium"),
    el!(77, "Ir", "Iridium"),
    el!(78, "Pt", "Platinum"),
    el!(79, "Au", "Gold"),
    el!(80, "Hg", "Mercury"),
    el!(81, "Tl", "Thallium"),
    el!(82, "Pb", "Lead"),
    el!(83, "Bi", "Bismuth"),
    el!(84, "Po", "Polonium"),
    el!(85, "At", "Astatine"),
    el!(86, "Rn", "Radon"),
    el!(87, "Fr", "Francium"),
    el!(88, "Ra", "Radium"),
    el!(89, "Ac", "Actinium"),
    el!(90, "Th", "Thorium"),
    el!(91, "Pa", "Protactinium"),
    el!(92, "U", "Uranium"),
    el!(93, "Np", "Neptunium"),
    el!(94, "Pu", "Plutonium"),
    el!(95, "Am", "Americium"),
    el!(96, "Cm", "Curium"),
    el!(97, "Bk", "Berkelium"),
    el!(98, "Cf", "Californium"),
    el!(99, "Es", "Einsteinium"),
    el!(100, "Fm", "Fermium"),
    el!(101, "Md", "Mendelevium"),
    el!(102, "No", "Nobelium"),
    el!(103, "Lr", "Lawrencium"),
    el!(104, "Rf", "Rutherfordium"),
    el!(105, "Db", "Dubnium"),
    el!(106, "Sg", "Seaborgium"),
    el!(107, "Bh", "Bohrium"),
    el!(108, "Hs", "Hassium"),
    el!(109, "Mt", "Meitnerium"),
    el!(110, "Ds", "Darmstadtium"),
    el!(111, "Rg", "Roentgenium"),
    el!(112, "Cn", "Copernicium"),
    el!(113, "Nh", "Nihonium"),
    el!(114, "Fl", "Flerovium"),
    el!(115, "Mc", "Moscovium"),
    el!(116, "Lv", "Livermorium"),
    el!(117, "Ts", "Tennessine"),
    el!(118, "Og", "Oganesson"),
];

pub fn element(number: u8) -> Option<&'static Element> {
    ELEMENTS.get(usize::from(number.checked_sub(1)?))
}

/// Zero-based row/column in an 18-column table with detached f-block rows 8/9.
pub fn table_position(number: u8) -> Option<(u8, u8)> {
    Some(match number {
        1 => (0, 0),
        2 => (0, 17),
        3..=4 => (1, number - 3),
        5..=10 => (1, number + 7),
        11..=12 => (2, number - 11),
        13..=18 => (2, number - 1),
        19..=36 => (3, number - 19),
        37..=54 => (4, number - 37),
        55..=56 => (5, number - 55),
        57..=71 => (8, number - 55),
        72..=86 => (5, number - 69),
        87..=88 => (6, number - 87),
        89..=103 => (9, number - 87),
        104..=118 => (6, number - 101),
        _ => return None,
    })
}

pub fn symbols(numbers: &BTreeSet<u8>) -> String {
    numbers
        .iter()
        .filter_map(|n| element(*n).map(|e| e.symbol))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Recognises symbols and balanced groups only. Counts are not evaluated: this
/// returns element identities, not stoichiometry, charge, phases or products.
pub fn formula_elements(text: &str) -> Option<BTreeSet<u8>> {
    let normalised: String = text
        .chars()
        .map(|c| match c {
            '₀' => '0',
            '₁' => '1',
            '₂' => '2',
            '₃' => '3',
            '₄' => '4',
            '₅' => '5',
            '₆' => '6',
            '₇' => '7',
            '₈' => '8',
            '₉' => '9',
            _ => c,
        })
        .collect();
    let mut chars = normalised.chars().peekable();
    let mut groups = Vec::new();
    let mut numbers = BTreeSet::new();
    while let Some(c) = chars.next() {
        if c.is_ascii_uppercase() {
            let mut symbol = c.to_string();
            if chars.peek().is_some_and(char::is_ascii_lowercase) {
                symbol.push(chars.next()?);
            }
            numbers.insert(ELEMENTS.iter().find(|e| e.symbol == symbol)?.number);
        } else {
            match c {
                '(' | '[' => groups.push(c),
                ')' if groups.pop() == Some('(') => {}
                ']' if groups.pop() == Some('[') => {}
                c if c.is_ascii_digit() || c.is_whitespace() => {}
                '+' | '-' | ',' | ';' | '.' | '·' => {}
                _ => return None,
            }
        }
    }
    (!numbers.is_empty() && groups.is_empty()).then_some(numbers)
}

// Small, explicit name vocabulary. No inference of unknown compound structures.
const NAME_PARTS: &[(&str, &[u8])] = &[
    ("aluminum", &[13]),
    ("sulphur", &[16]),
    ("cesium", &[55]),
    ("wolfram", &[74]),
    ("oxide", &[8]),
    ("sulfide", &[16]),
    ("sulphide", &[16]),
    ("selenide", &[34]),
    ("telluride", &[52]),
    ("nitride", &[7]),
    ("phosphide", &[15]),
    ("arsenide", &[33]),
    ("fluoride", &[9]),
    ("chloride", &[17]),
    ("bromide", &[35]),
    ("iodide", &[53]),
    ("hydride", &[1]),
    ("carbide", &[6]),
    ("hydroxide", &[1, 8]),
    ("sulfate", &[16, 8]),
    ("sulphate", &[16, 8]),
    ("nitrate", &[7, 8]),
    ("carbonate", &[6, 8]),
    ("phosphate", &[15, 8]),
    ("hydrate", &[1, 8]),
    ("water", &[1, 8]),
    ("ammonia", &[1, 7]),
    ("methane", &[1, 6]),
    ("ethanol", &[1, 6, 8]),
    ("acetone", &[1, 6, 8]),
];

pub fn suggest_elements(text: &str) -> BTreeSet<u8> {
    if let Some(numbers) = formula_elements(text) {
        return numbers;
    }
    let lower = text.to_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();
    let mut result = BTreeSet::new();
    for word in &words {
        if let Some(element) = ELEMENTS.iter().find(|e| e.name.eq_ignore_ascii_case(word)) {
            result.insert(element.number);
        }
        let part = NAME_PARTS
            .iter()
            .find(|(name, _)| name == word)
            .or_else(|| {
                [
                    "mono", "di", "tri", "tetra", "penta", "hexa", "hepta", "octa", "nona", "deca",
                ]
                .iter()
                .find_map(|prefix| {
                    word.strip_prefix(prefix)
                        .and_then(|rest| NAME_PARTS.iter().find(|(name, _)| *name == rest))
                })
            });
        if let Some((_, numbers)) = part {
            result.extend(*numbers);
        }
    }
    // Prefix completion applies to the last word, not arbitrary substrings.
    if let Some(last) = words.last().filter(|word| {
        word.len() >= 2
            && text
                .trim_end()
                .chars()
                .last()
                .is_some_and(char::is_alphabetic)
    }) {
        for element in &ELEMENTS {
            if element.name.to_ascii_lowercase().starts_with(last) {
                result.insert(element.number);
            }
        }
        for (name, numbers) in NAME_PARTS
            .iter()
            .filter(|(name, _)| ["aluminum", "sulphur", "cesium", "wolfram"].contains(name))
        {
            if name.starts_with(last) {
                result.extend(*numbers);
            }
        }
    }
    result
}
