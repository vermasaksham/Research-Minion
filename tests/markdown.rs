use crystal_forge::{
    editor::Editor,
    markdown::{self, Format},
    store::Store,
};

#[test]
fn inline_formatting_preserves_unicode_and_selects_the_original_text() {
    for (action, expected, selected) in [
        (Format::Bold, "ΔT **50 °C 🧪**; Sb₂Se₃", 5..12),
        (Format::Italic, "ΔT *50 °C 🧪*; Sb₂Se₃", 4..11),
    ] {
        let mut body = "ΔT 50 °C 🧪; Sb₂Se₃".to_string();
        let range = markdown::format(&mut body, 3..10, action);
        assert_eq!(body, expected);
        assert_eq!(range, selected);
        assert_eq!(
            body.chars()
                .skip(range.start)
                .take(range.len())
                .collect::<String>(),
            "50 °C 🧪"
        );
    }
}

#[test]
fn scientific_script_buttons_keep_formula_characters_readable() {
    let mut body = "Sb2Se3 and cm-1".to_string();
    markdown::format(&mut body, 2..3, Format::Subscript);
    let second = body.find('3').unwrap();
    markdown::format(&mut body, second..second + 1, Format::Subscript);
    let power = body.find("-1").unwrap();
    markdown::format(&mut body, power..power + 2, Format::Superscript);
    assert_eq!(body, "Sb<sub>2</sub>Se<sub>3</sub> and cm<sup>-1</sup>");
    let segments = markdown::script_spans(&body).unwrap();
    assert_eq!(
        segments.iter().filter(|s| s.raised == Some(false)).count(),
        2
    );
    assert_eq!(
        segments.iter().filter(|s| s.raised == Some(true)).count(),
        1
    );
}

#[test]
fn script_buttons_format_whole_words_and_unicode_without_losing_source() {
    let mut body = "phase alpha and β₂".to_string();
    let selection = markdown::format(&mut body, 6..11, Format::Superscript);
    assert_eq!(body, "phase <sup>alpha</sup> and β₂");
    assert_eq!(selection, 11..16);
    let start = body.chars().position(|c| c == 'β').unwrap();
    markdown::format(&mut body, start..start + 2, Format::Subscript);
    assert_eq!(body, "phase <sup>alpha</sup> and <sub>β₂</sub>");
    let spans = markdown::script_spans(&body).unwrap();
    assert_eq!(spans[1].text, "alpha");
    assert_eq!(spans[1].raised, Some(true));
    assert_eq!(spans[3].text, "β₂");
    assert_eq!(spans[3].raised, Some(false));
    assert!(markdown::script_spans("keep <sup>unfinished as plain text").is_none());
}

#[test]
fn empty_and_stale_selections_insert_editable_placeholders_without_panicking() {
    let mut body = "Se".to_string();
    assert_eq!(markdown::format(&mut body, 100..100, Format::Bold), 4..8);
    assert_eq!(body, "Se**text**");
    let mut body = "Sb".to_string();
    assert_eq!(
        markdown::format(
            &mut body,
            std::ops::Range { start: 2, end: 0 },
            Format::Italic
        ),
        1..3
    );
    assert_eq!(body, "*Sb*");
}

#[test]
fn line_formatting_uses_whole_touched_lines_and_preserves_the_next_line() {
    for (action, expected) in [
        (Format::Heading, "## Source\n## Sink\nResult"),
        (Format::Bullets, "- Source\n- Sink\nResult"),
        (Format::Numbered, "1. Source\n2. Sink\nResult"),
        (Format::Quote, "> Source\n> Sink\nResult"),
    ] {
        let mut body = "Source\nSink\nResult".to_string();
        let range = markdown::format(&mut body, 2..12, action);
        assert_eq!(body, expected);
        assert_eq!(range.start, 0);
        assert_eq!(&body[range.end..], "\nResult");
    }
    let mut body = "first\r\nsecond".to_string();
    markdown::format(&mut body, 0..7, Format::Bullets);
    assert_eq!(body, "- first\r\nsecond");
}

#[test]
fn code_formatting_handles_literal_backticks_and_multiline_blocks() {
    let mut body = "`x`".to_string();
    let range = markdown::format(&mut body, 0..3, Format::Code);
    assert_eq!(body, "`` `x` ``");
    assert_eq!(range, 3..6);
    let mut body = "prefix a\n```\nb suffix".to_string();
    let range = markdown::format(&mut body, 7..14, Format::Code);
    assert_eq!(body, "prefix \n````\na\n```\nb\n````\n suffix");
    assert_eq!(
        body.chars()
            .skip(range.start)
            .take(range.len())
            .collect::<String>(),
        "a\n```\nb"
    );
}

#[test]
fn link_action_preserves_the_label_and_selects_only_the_destination() {
    let mut body = "See ΔT data".to_string();
    let range = markdown::format(&mut body, 4..6, Format::Link);
    assert_eq!(body, "See [ΔT](https://example.com) data");
    assert_eq!(
        body.chars()
            .skip(range.start)
            .take(range.len())
            .collect::<String>(),
        "https://example.com"
    );
}

#[test]
fn markdown_source_survives_edit_save_search_and_reopen_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let source = "# Growth notes\r\n\r\n**ΔT** = 50 °C; Sb₂Se₃\n\n- [x] Inspected\n\n| Source | Sink |\n|---|---|\n|650|600|\n\n```text\n  x = 1\t# preserve whitespace\n```\n\n[Reference](https://example.com)\n";
    let id = {
        let store = Store::open(dir.path()).unwrap();
        let project = store.create_project("Markdown", "").unwrap();
        let mut editor = Editor::new(project.id);
        editor.title = "Growth observations".into();
        editor.body = source.into();
        assert!(editor.is_dirty());
        editor.save(&store).unwrap();
        assert!(!editor.is_dirty());
        assert_eq!(editor.body, source);
        editor.saved.unwrap().id
    };
    let store = Store::open(dir.path()).unwrap();
    let entry = store.entry(&id).unwrap();
    assert_eq!(entry.body.as_bytes(), source.as_bytes());
    assert_eq!(store.entries(&entry.project_id, "Sb₂Se₃").unwrap().len(), 1);
}
