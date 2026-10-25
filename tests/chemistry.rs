use research_minion::chemistry::{
    ELEMENTS, element, formula_elements, suggest_elements, table_position,
};
use std::collections::BTreeSet;

fn set(numbers: &[u8]) -> BTreeSet<u8> {
    numbers.iter().copied().collect()
}

#[test]
fn periodic_table_contains_118_unique_correctly_addressable_elements() {
    assert_eq!(ELEMENTS.len(), 118);
    assert_eq!(
        ELEMENTS
            .iter()
            .map(|e| e.symbol)
            .collect::<BTreeSet<_>>()
            .len(),
        118
    );
    let mut positions = BTreeSet::new();
    for (index, e) in ELEMENTS.iter().enumerate() {
        assert_eq!(usize::from(e.number), index + 1);
        assert_eq!(element(e.number).unwrap().symbol, e.symbol);
        let (row, col) = table_position(e.number).unwrap();
        assert!(row <= 9 && col < 18 && row != 7);
        assert!(positions.insert((row, col)));
    }
    assert_eq!(element(51).unwrap().name, "Antimony");
    assert_eq!(element(34).unwrap().symbol, "Se");
    assert_eq!(element(118).unwrap().name, "Oganesson");
    assert_eq!(table_position(2), Some((0, 17)));
    assert_eq!(table_position(57), Some((8, 2)));
    assert_eq!(table_position(71), Some((8, 16)));
    assert_eq!(table_position(89), Some((9, 2)));
    assert_eq!(table_position(103), Some((9, 16)));
    assert!(element(0).is_none() && element(119).is_none());
}

#[test]
fn chemical_formulas_resolve_elements_including_groups_and_subscripts() {
    for (formula, numbers) in [
        ("Sb2Se3", vec![34, 51]),
        ("Sb₂Se₃", vec![34, 51]),
        ("Sb + Se", vec![34, 51]),
        ("CuSO4·5H2O", vec![1, 8, 16, 29]),
        ("K4[Fe(CN)6]", vec![6, 7, 19, 26]),
        ("Ca(OH)2", vec![1, 8, 20]),
        ("2 H2O", vec![1, 8]),
        ("I2", vec![53]),
    ] {
        assert_eq!(formula_elements(formula), Some(set(&numbers)), "{formula}");
    }
}

#[test]
fn formula_case_distinguishes_cobalt_from_carbon_and_oxygen() {
    assert_eq!(suggest_elements("Co"), set(&[27]));
    assert_eq!(suggest_elements("CO"), set(&[6, 8]));
    assert_eq!(suggest_elements("CO2"), set(&[6, 8]));
}

#[test]
fn names_and_incomplete_element_names_have_local_suggestions() {
    for (name, numbers) in [
        ("antimony selenide", vec![34, 51]),
        ("antimony triselenide", vec![34, 51]),
        ("molybdenum disulfide", vec![16, 42]),
        ("lead(II) iodide", vec![53, 82]),
        ("copper sulfate pentahydrate", vec![1, 8, 16, 29]),
        ("aluminum oxide", vec![8, 13]),
        ("cesium chloride", vec![17, 55]),
        ("water", vec![1, 8]),
        ("anti", vec![51]),
    ] {
        assert_eq!(suggest_elements(name), set(&numbers), "{name}");
    }
}

#[test]
fn invalid_or_unknown_text_does_not_invent_a_compound() {
    for formula in [
        "Xx2Se3",
        "Fe(OH]2",
        "Sb(Se3",
        "mystery solvent",
        "silverfish",
        "",
    ] {
        assert!(formula_elements(formula).is_none(), "{formula}");
        assert!(suggest_elements(formula).is_empty(), "{formula}");
    }
}
