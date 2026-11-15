//! Copy an OOXML workbook and append a calculation worksheet. Existing sheet
//! XML, formulas, styles and layout are copied byte-for-byte after decompression.
//! No template is bundled or changed in place.
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XlsxError {
    #[error("Template workbook error: {0}")]
    Invalid(String),
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, XlsxError>;

#[derive(Clone, Debug)]
pub struct ExportRow {
    pub temperature_k: f64,
    pub delta_h_j_mol: f64,
    pub delta_s_j_mol_k: f64,
    pub delta_g_j_mol: f64,
    pub log10_k: f64,
    pub gas_pressure_bar: Option<f64>,
    pub log10_gas_pressure: Option<f64>,
    pub electrode_potential_v: Option<f64>,
    pub phases: String,
    pub provenance: String,
}

struct Part {
    name: String,
    method: u16,
    compressed: Vec<u8>,
    uncompressed_size: u32,
    crc: u32,
}
fn u16_at(data: &[u8], at: usize) -> Result<u16> {
    let x = data
        .get(at..at + 2)
        .ok_or_else(|| XlsxError::Invalid("Truncated ZIP field.".into()))?;
    Ok(u16::from_le_bytes([x[0], x[1]]))
}
fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    let x = data
        .get(at..at + 4)
        .ok_or_else(|| XlsxError::Invalid("Truncated ZIP field.".into()))?;
    Ok(u32::from_le_bytes([x[0], x[1], x[2], x[3]]))
}
fn as_usize(value: u32) -> Result<usize> {
    usize::try_from(value).map_err(|_| XlsxError::Invalid("ZIP offset too large.".into()))
}

fn read_parts(data: &[u8]) -> Result<Vec<Part>> {
    if data.len() > 80_000_000 {
        return Err(XlsxError::Invalid(
            "Template exceeds the 80 MB limit.".into(),
        ));
    }
    let eocd = (0..data.len().saturating_sub(3))
        .rev()
        .find(|&at| data.get(at..at + 4) == Some(b"PK\x05\x06"))
        .ok_or_else(|| XlsxError::Invalid("Missing ZIP end record.".into()))?;
    if u16_at(data, eocd + 4)? != 0 || u16_at(data, eocd + 6)? != 0 {
        return Err(XlsxError::Invalid(
            "Multi-disk ZIP files are unsupported.".into(),
        ));
    }
    let count = usize::from(u16_at(data, eocd + 10)?);
    let mut cursor = as_usize(u32_at(data, eocd + 16)?)?;
    let mut parts = Vec::with_capacity(count);
    for _ in 0..count {
        if data.get(cursor..cursor + 4) != Some(b"PK\x01\x02") {
            return Err(XlsxError::Invalid("Invalid ZIP directory.".into()));
        }
        let flags = u16_at(data, cursor + 8)?;
        if flags & 1 != 0 {
            return Err(XlsxError::Invalid(
                "Encrypted ZIP entries are unsupported.".into(),
            ));
        }
        let method = u16_at(data, cursor + 10)?;
        if method != 0 && method != 8 {
            return Err(XlsxError::Invalid(
                "Unsupported ZIP compression method.".into(),
            ));
        }
        let crc = u32_at(data, cursor + 16)?;
        let compressed_size = u32_at(data, cursor + 20)?;
        let uncompressed_size = u32_at(data, cursor + 24)?;
        let name_len = usize::from(u16_at(data, cursor + 28)?);
        let extra_len = usize::from(u16_at(data, cursor + 30)?);
        let comment_len = usize::from(u16_at(data, cursor + 32)?);
        let local = as_usize(u32_at(data, cursor + 42)?)?;
        let name = String::from_utf8(
            data.get(cursor + 46..cursor + 46 + name_len)
                .ok_or_else(|| XlsxError::Invalid("Truncated ZIP name.".into()))?
                .to_vec(),
        )
        .map_err(|_| XlsxError::Invalid("Non-UTF-8 ZIP entry name.".into()))?;
        if data.get(local..local + 4) != Some(b"PK\x03\x04") {
            return Err(XlsxError::Invalid("Invalid ZIP local entry.".into()));
        }
        let start = local
            + 30
            + usize::from(u16_at(data, local + 26)?)
            + usize::from(u16_at(data, local + 28)?);
        let end = start
            .checked_add(as_usize(compressed_size)?)
            .ok_or_else(|| XlsxError::Invalid("ZIP entry overflow.".into()))?;
        let compressed = data
            .get(start..end)
            .ok_or_else(|| XlsxError::Invalid("Truncated ZIP data.".into()))?
            .to_vec();
        parts.push(Part {
            name,
            method,
            compressed,
            uncompressed_size,
            crc,
        });
        cursor = cursor
            .checked_add(46 + name_len + extra_len + comment_len)
            .ok_or_else(|| XlsxError::Invalid("ZIP directory overflow.".into()))?;
    }
    Ok(parts)
}

fn decoded(part: &Part) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    if part.method == 0 {
        output.extend_from_slice(&part.compressed);
    } else {
        DeflateDecoder::new(part.compressed.as_slice())
            .take(20_000_001)
            .read_to_end(&mut output)?;
    }
    if output.len() != part.uncompressed_size as usize
        || crc32fast::hash(&output) != part.crc
        || output.len() > 20_000_000
    {
        return Err(XlsxError::Invalid(format!(
            "Corrupt or oversized ZIP entry {}.",
            part.name
        )));
    }
    Ok(output)
}

fn replace_part(
    parts: &mut [Part],
    name: &str,
    modify: impl FnOnce(String) -> Result<String>,
) -> Result<()> {
    let part = parts
        .iter_mut()
        .find(|part| part.name == name)
        .ok_or_else(|| XlsxError::Invalid(format!("Missing {name}.")))?;
    let old = String::from_utf8(decoded(part)?)
        .map_err(|_| XlsxError::Invalid(format!("Invalid XML in {name}.")))?;
    let new = modify(old)?;
    *part = make_part(name, &new)?;
    Ok(())
}
fn make_part(name: &str, content: &str) -> Result<Part> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content.as_bytes())?;
    let compressed = encoder.finish()?;
    let uncompressed_size = u32::try_from(content.len())
        .map_err(|_| XlsxError::Invalid("XML part too large.".into()))?;
    Ok(Part {
        name: name.into(),
        method: 8,
        compressed,
        uncompressed_size,
        crc: crc32fast::hash(content.as_bytes()),
    })
}
fn insert_before(mut xml: String, closing: &str, fragment: &str) -> Result<String> {
    let at = xml
        .rfind(closing)
        .ok_or_else(|| XlsxError::Invalid(format!("Missing XML closing tag {closing}.")))?;
    xml.insert_str(at, fragment);
    Ok(xml)
}
fn max_number_after(xml: &str, pattern: &str) -> u32 {
    xml.match_indices(pattern)
        .filter_map(|(at, _)| {
            xml[at + pattern.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0)
}
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn text_cell(address: &str, value: &str) -> String {
    format!(
        "<c r=\"{address}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
        xml_escape(value)
    )
}
fn number_cell(address: &str, value: f64) -> Result<String> {
    if !value.is_finite() {
        return Err(XlsxError::Invalid("A result is not finite.".into()));
    }
    Ok(format!("<c r=\"{address}\"><v>{value}</v></c>"))
}
fn write_archive(parts: &[Part]) -> Result<Vec<u8>> {
    if parts.len() > u16::MAX as usize {
        return Err(XlsxError::Invalid("Too many ZIP parts.".into()));
    }
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for part in parts {
        let offset = u32::try_from(out.len())
            .map_err(|_| XlsxError::Invalid("ZIP archive too large.".into()))?;
        let name = part.name.as_bytes();
        let name_len = u16::try_from(name.len())
            .map_err(|_| XlsxError::Invalid("ZIP name too long.".into()))?;
        let compressed_size = u32::try_from(part.compressed.len())
            .map_err(|_| XlsxError::Invalid("ZIP entry too large.".into()))?;
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&part.method.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&part.crc.to_le_bytes());
        out.extend_from_slice(&compressed_size.to_le_bytes());
        out.extend_from_slice(&part.uncompressed_size.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&part.compressed);
        directory.extend_from_slice(b"PK\x01\x02");
        directory.extend_from_slice(&20_u16.to_le_bytes());
        directory.extend_from_slice(&20_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&part.method.to_le_bytes());
        directory.extend_from_slice(&0_u32.to_le_bytes());
        directory.extend_from_slice(&part.crc.to_le_bytes());
        directory.extend_from_slice(&compressed_size.to_le_bytes());
        directory.extend_from_slice(&part.uncompressed_size.to_le_bytes());
        directory.extend_from_slice(&name_len.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u16.to_le_bytes());
        directory.extend_from_slice(&0_u32.to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name);
    }
    let directory_offset = u32::try_from(out.len())
        .map_err(|_| XlsxError::Invalid("ZIP archive too large.".into()))?;
    let directory_len = u32::try_from(directory.len())
        .map_err(|_| XlsxError::Invalid("ZIP directory too large.".into()))?;
    out.extend_from_slice(&directory);
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0_u32.to_le_bytes());
    let count = parts.len() as u16;
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&directory_len.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    Ok(out)
}

/// Creates a new .xlsx file from a user-provided template; the template is read-only.
pub fn export_template(
    template: &Path,
    destination: &Path,
    name: &str,
    equation: &str,
    source: &str,
    rows: &[ExportRow],
) -> Result<()> {
    if template == destination
        || (destination.exists() && fs::canonicalize(template)? == fs::canonicalize(destination)?)
    {
        return Err(XlsxError::Invalid(
            "Choose an output path different from the template.".into(),
        ));
    }
    if rows.is_empty() {
        return Err(XlsxError::Invalid(
            "Calculate results before exporting.".into(),
        ));
    }
    let mut parts = read_parts(&fs::read(template)?)?;
    let workbook = parts
        .iter()
        .find(|part| part.name == "xl/workbook.xml")
        .ok_or_else(|| XlsxError::Invalid("No workbook.xml.".into()))?;
    let workbook_xml = String::from_utf8(decoded(workbook)?)
        .map_err(|_| XlsxError::Invalid("Invalid workbook XML.".into()))?;
    let sheet_id = max_number_after(&workbook_xml, "sheetId=\"") + 1;
    let existing_name = "CrystalForge Results";
    let sheet_name = if workbook_xml.contains("name=\"CrystalForge Results\"") {
        let suffix = sheet_id.to_string();
        format!("CrystalForge {}", suffix)
    } else {
        existing_name.into()
    };
    let rel = parts
        .iter()
        .find(|part| part.name == "xl/_rels/workbook.xml.rels")
        .ok_or_else(|| XlsxError::Invalid("No workbook relationships.".into()))?;
    let rel_xml = String::from_utf8(decoded(rel)?)
        .map_err(|_| XlsxError::Invalid("Invalid relationships XML.".into()))?;
    let rel_id = max_number_after(&rel_xml, "Id=\"rId") + 1;
    let sheet_number = (1..)
        .find(|n| {
            !parts
                .iter()
                .any(|p| p.name == format!("xl/worksheets/sheet{n}.xml"))
        })
        .unwrap();
    let path = format!("xl/worksheets/sheet{sheet_number}.xml");
    replace_part(&mut parts, "xl/workbook.xml", |xml| {
        insert_before(
            xml,
            "</sheets>",
            &format!(
                "<sheet xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" name=\"{}\" sheetId=\"{sheet_id}\" r:id=\"rId{rel_id}\"/>",
                xml_escape(&sheet_name)
            ),
        )
    })?;
    replace_part(&mut parts, "xl/_rels/workbook.xml.rels", |xml| {
        insert_before(
            xml,
            "</Relationships>",
            &format!(
                "<Relationship Id=\"rId{rel_id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{sheet_number}.xml\"/>"
            ),
        )
    })?;
    replace_part(&mut parts, "[Content_Types].xml", |xml| {
        insert_before(
            xml,
            "</Types>",
            &format!(
                "<Override PartName=\"/{path}\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"
            ),
        )
    })?;
    let headings = [
        "T / K",
        "ΔH° / J mol⁻¹",
        "ΔS° / J mol⁻¹ K⁻¹",
        "ΔG° / J mol⁻¹",
        "log₁₀ K",
        "p(gas) / bar",
        "log₁₀(p/p°)",
        "E° / V",
        "Selected phases",
        "Species provenance",
    ];
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
    );
    for (row, label) in [
        (1, format!("CrystalForge | {name}")),
        (2, format!("Reaction: {equation}")),
        (3, format!("Source: {source}")),
    ] {
        xml.push_str(&format!(
            "<row r=\"{row}\">{}</row>",
            text_cell(&format!("A{row}"), &label)
        ));
    }
    xml.push_str("<row r=\"5\">");
    for (column, heading) in headings.iter().enumerate() {
        xml.push_str(&text_cell(
            &format!("{}5", (b'A' + column as u8) as char),
            heading,
        ));
    }
    xml.push_str("</row>");
    for (i, row) in rows.iter().enumerate() {
        let index = i + 6;
        xml.push_str(&format!("<row r=\"{index}\">"));
        for (column, value) in [
            ("A", row.temperature_k),
            ("B", row.delta_h_j_mol),
            ("C", row.delta_s_j_mol_k),
            ("D", row.delta_g_j_mol),
            ("E", row.log10_k),
        ] {
            xml.push_str(&number_cell(&format!("{column}{index}"), value)?);
        }
        for (column, value) in [
            ("F", row.gas_pressure_bar),
            ("G", row.log10_gas_pressure),
            ("H", row.electrode_potential_v),
        ] {
            if let Some(value) = value {
                xml.push_str(&number_cell(&format!("{column}{index}"), value)?);
            }
        }
        xml.push_str(&text_cell(&format!("I{index}"), &row.phases));
        xml.push_str(&text_cell(&format!("J{index}"), &row.provenance));
        xml.push_str("</row>");
    }
    xml.push_str("</sheetData></worksheet>");
    parts.push(make_part(&path, &xml)?);
    let output = write_archive(&parts)?;
    fs::write(destination, output)?;
    Ok(())
}
