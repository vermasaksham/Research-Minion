//! Source-preserving Markdown formatting with character-based selections.
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub enum Format {
    Bold,
    Italic,
    Heading,
    Bullets,
    Numbered,
    Quote,
    Code,
    Link,
    Subscript,
    Superscript,
}

/// Apply a toolbar action and return the selection in the resulting source.
/// Egui cursor positions count Unicode characters, not UTF-8 bytes.
pub fn format(body: &mut String, selection: Range<usize>, action: Format) -> Range<usize> {
    let count = body.chars().count();
    let start = selection.start.min(selection.end).min(count);
    let end = selection.start.max(selection.end).min(count);
    let byte = |index| {
        body.char_indices()
            .nth(index)
            .map_or(body.len(), |(i, _)| i)
    };
    let (mut from, mut to) = (byte(start), byte(end));
    if matches!(
        action,
        Format::Heading | Format::Bullets | Format::Numbered | Format::Quote
    ) {
        from = body[..from].rfind('\n').map_or(0, |i| i + 1);
        // A selection ending at the next line's start does not include that line.
        if to > from && body[..to].ends_with('\n') {
            to -= 1;
        } else {
            to = body[to..].find('\n').map_or(body.len(), |i| to + i);
        }
        let replacement = body[from..to]
            .split('\n')
            .enumerate()
            .map(|(i, line)| {
                let prefix = match action {
                    Format::Heading => "## ".into(),
                    Format::Bullets => "- ".into(),
                    Format::Numbered => format!("{}. ", i + 1),
                    Format::Quote => "> ".into(),
                    _ => unreachable!(),
                };
                format!("{prefix}{line}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let selection_start = body[..from].chars().count();
        let selection_end = selection_start + replacement.chars().count();
        body.replace_range(from..to, &replacement);
        return selection_start..selection_end;
    }
    let selected = &body[from..to];
    let text = if selected.is_empty() {
        "text"
    } else {
        selected
    };
    let (replacement, offset, length) = match action {
        Format::Subscript | Format::Superscript => {
            let converted: String = text
                .chars()
                .map(|c| script_char(c, matches!(action, Format::Superscript)).unwrap_or(c))
                .collect();
            (converted.clone(), 0, converted.chars().count())
        }
        Format::Bold => (format!("**{text}**"), 2, text.chars().count()),
        Format::Italic => (format!("*{text}*"), 1, text.chars().count()),
        Format::Link => {
            let label_len = text.chars().count();
            (format!("[{text}](https://example.com)"), label_len + 3, 19)
        }
        Format::Code => {
            let run = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
            if text.contains('\n') {
                let fence = "`".repeat((run + 1).max(3));
                let prefix = if from == 0 || body[..from].ends_with('\n') {
                    ""
                } else {
                    "\n"
                };
                let suffix = if to == body.len() || body[to..].starts_with('\n') {
                    ""
                } else {
                    "\n"
                };
                let offset = prefix.len() + fence.len() + 1;
                (
                    format!("{prefix}{fence}\n{text}\n{fence}{suffix}"),
                    offset,
                    text.chars().count(),
                )
            } else {
                let fence = "`".repeat(run + 1);
                let pad = if text.starts_with('`') || text.ends_with('`') {
                    " "
                } else {
                    ""
                };
                let offset = fence.len() + pad.len();
                (
                    format!("{fence}{pad}{text}{pad}{fence}"),
                    offset,
                    text.chars().count(),
                )
            }
        }
        _ => unreachable!(),
    };
    body.replace_range(from..to, &replacement);
    (start + offset)..(start + offset + length)
}

fn script_char(c: char, superscript: bool) -> Option<char> {
    let plain = "0123456789+-=()";
    let raised = "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾";
    let lowered = "₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎";
    plain.chars().position(|item| item == c).and_then(|index| {
        (if superscript { raised } else { lowered })
            .chars()
            .nth(index)
    })
}
