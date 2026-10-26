# Markdown research notes

Open an experiment and a note. **Write** edits its Markdown source; **Preview**
renders the current draft. Switching views does not save or discard changes.
Click **Save entry** or press **Ctrl+S** to save the exact source locally.
Existing notes keep their original text.

Select text and click **Bold**, **Italic**, **Heading**, **Bullets**, **1. List**,
**Quote**, **Code**, **Link**, **Subscript**, or **Superscript**. With no selection, inline buttons insert an
editable placeholder. Heading/list/quote actions prefix the current line or all
selected lines. Link selects the destination so you can replace it immediately.
Code uses a fenced block for a multiline selection. Undo/redo works with the
formatting buttons. While writing, **Ctrl+B**, **Ctrl+I** and **Ctrl+K** apply bold,
italic and link formatting.

Subscript and superscript turn selected digits and common signs into Unicode
characters, for example `Sb₂Se₃` and `cm⁻¹`. Other letters stay as typed.
On Windows, **Add picture…** chooses a PNG or JPEG and copies a bounded PNG into
the active profile's `note-images` folder. It adds a reference at the end of the
draft. Save the note, then use Preview to see the picture below the text. Back up
the entire profile folder to keep its images. Removing a note reference does not
delete the copied picture file.

You can also type Markdown directly:

````markdown
# Growth observations

**Material:** Sb₂Se₃ · *Thermal character:* researcher-assigned

- Source: **650 °C**
- Sink: **600 °C**
- ΔT: **50 °C**

| Parameter | Recorded value |
| --- | --- |
| Transport agent | I₂ |

> Keep observations separate from interpretation.

- [x] Record temperatures
- [ ] Inspect morphology

```text
Source - Sink = 650 - 600 = 50 °C
```

[Reference](https://example.com)

~~Superseded interpretation~~
````

Task markers are read-only in Preview; change `[ ]` to `[x]` in Write. Links open
in the system browser only when clicked. Preview shows locally added pictures;
arbitrary external images and raw HTML are not loaded.
Mathematical notation can be recorded as source text; there is
no LaTeX equation renderer. A preview is formatting, not scientific validation.

Search still searches the saved source. Previewing a draft does not change the
saved record, its revision, or another profile's notes. Normal navigation and
close protection continue to apply to unsaved Markdown edits.
