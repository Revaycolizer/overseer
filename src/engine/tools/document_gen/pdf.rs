use super::utils::find_system_font;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, Stream,
};
use serde_json::Value;
use skrifa::{
    instance::{LocationRef, Size},
    FontRef, MetadataProvider,
};
use std::{collections::BTreeMap, path::PathBuf};

const PAGE_WIDTH: f32 = 595.28;
const PAGE_HEIGHT: f32 = 841.89;
const MARGIN: f32 = 42.52; // 15 mm
const FONT_SIZE: f32 = 11.0;
const LINE_HEIGHT: f32 = 16.0;

pub fn generate_pdf(
    dir: &PathBuf,
    prefix: &str,
    headers: Option<&[String]>,
    data: Option<&Vec<Value>>,
    content: Option<&str>,
) -> Result<String, String> {
    let filename = format!("{}.pdf", prefix);
    let mut paragraphs: Vec<String> = content
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    if let (Some(headers), Some(rows)) = (headers, data) {
        if !headers.is_empty() {
            paragraphs.push(headers.join(" | "));
            for row in rows {
                let cells: Vec<String> = headers
                    .iter()
                    .enumerate()
                    .map(|(index, header)| {
                        let value = if row.is_object() {
                            row.get(header)
                        } else {
                            row.get(index)
                        };
                        match value {
                            None | Some(Value::Null) => String::new(),
                            Some(Value::String(text)) => text.clone(),
                            Some(value) => value.to_string(),
                        }
                    })
                    .collect();
                paragraphs.push(cells.join(" | "));
            }
        }
    }
    let bytes = find_system_font()?;
    let mut document = build_document(&bytes, &paragraphs)?;
    document.compress();
    document
        .save(dir.join(&filename))
        .map_err(|error| error.to_string())?;
    Ok(filename)
}

/// Embed a TrueType font with separate CIDs for each Unicode character. A
/// ToUnicode map preserves copy/paste and extraction, including surrogate pairs.
fn build_document(font_bytes: &[u8], paragraphs: &[String]) -> Result<Document, String> {
    // Invisible control bytes in user text must not become missing CIDs.
    let paragraphs: Vec<String> = paragraphs
        .iter()
        .map(|text| {
            text.chars()
                .map(|ch| if ch.is_control() { ' ' } else { ch })
                .collect()
        })
        .collect();
    let font = FontRef::new(font_bytes).map_err(|error| error.to_string())?;
    let metrics = font.metrics(Size::unscaled(), LocationRef::default());
    if metrics.units_per_em == 0 {
        return Err("Font has invalid metrics".into());
    }
    let scale = 1000.0 / metrics.units_per_em as f32;
    let charmap = font.charmap();
    let glyph_metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    let mut characters = BTreeMap::new();
    for character in paragraphs
        .iter()
        .flat_map(|text| text.chars())
        .chain(std::iter::once(' '))
    {
        if character.is_control() || characters.contains_key(&character) {
            continue;
        }
        let cid = u16::try_from(characters.len() + 1)
            .map_err(|_| "Too many distinct characters in PDF")?;
        let glyph = charmap
            .map(character)
            .ok_or_else(|| format!("PDF font does not support character {character:?}"))?;
        let width = glyph_metrics.advance_width(glyph).unwrap_or(0.0) * scale;
        characters.insert(character, (cid, glyph.to_u32() as u16, width));
    }
    let mut lines = Vec::new();
    let max_width = PAGE_WIDTH - 2.0 * MARGIN;
    for paragraph in &paragraphs {
        let mut line = String::new();
        let mut width = 0.0;
        // Wrap at word boundaries, splitting exceptionally long words by glyph.
        for word in paragraph.split_whitespace() {
            let word_width: f32 = word
                .chars()
                .map(|ch| characters[&ch].2 * FONT_SIZE / 1000.0)
                .sum();
            let space_width = characters[&' '].2 * FONT_SIZE / 1000.0;
            if !line.is_empty() && width + space_width + word_width > max_width {
                lines.push(std::mem::take(&mut line));
                width = 0.0;
            }
            if !line.is_empty() {
                line.push(' ');
                width += space_width;
            }
            for ch in word.chars() {
                let advance = characters[&ch].2 * FONT_SIZE / 1000.0;
                if !line.is_empty() && width + advance > max_width {
                    lines.push(std::mem::take(&mut line));
                    width = 0.0;
                }
                line.push(ch);
                width += advance;
            }
        }
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    let mut document = Document::with_version("1.7");
    let font_file = document.add_object(Stream::new(
        dictionary! {"Length1" => font_bytes.len() as i64},
        font_bytes.to_vec(),
    ));
    let bounds = metrics.bounds.unwrap_or_default();
    let descriptor = document.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => "OverseerFont", "Flags" => 32,
        "FontBBox" => vec![bounds.x_min * scale, bounds.y_min * scale, bounds.x_max * scale, bounds.y_max * scale].into_iter().map(Object::from).collect::<Vec<_>>(),
        "ItalicAngle" => metrics.italic_angle, "Ascent" => metrics.ascent * scale,
        "Descent" => metrics.descent * scale, "CapHeight" => metrics.cap_height.unwrap_or(metrics.ascent) * scale,
        "StemV" => 80, "FontFile2" => font_file,
    });
    let mut cid_to_gid = vec![0; (characters.len() + 1) * 2];
    let mut widths = Vec::new();
    let mut mappings = Vec::new();
    for (ch, (cid, gid, width)) in &characters {
        cid_to_gid[*cid as usize * 2..*cid as usize * 2 + 2].copy_from_slice(&gid.to_be_bytes());
        widths.push(Object::Integer(*cid as i64));
        widths.push(Object::Array(vec![Object::Real(*width)]));
        let unicode: String = ch
            .encode_utf16(&mut [0; 2])
            .iter()
            .map(|unit| format!("{unit:04X}"))
            .collect();
        mappings.push(format!("<{cid:04X}> <{unicode}>\n"));
    }
    let gid_map = document.add_object(Stream::new(dictionary! {}, cid_to_gid));
    let descendant = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "OverseerFont",
        "CIDSystemInfo" => dictionary! {"Registry" => Object::string_literal("Adobe"), "Ordering" => Object::string_literal("Identity"), "Supplement" => 0},
        "FontDescriptor" => descriptor, "CIDToGIDMap" => gid_map, "W" => widths,
    });
    let mut cmap = String::from("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /OverseerUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
    for chunk in mappings.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for mapping in chunk {
            cmap.push_str(mapping);
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    let unicode_map = document.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
    let type0 = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "OverseerFont",
        "Encoding" => "Identity-H", "DescendantFonts" => vec![Object::Reference(descendant)], "ToUnicode" => unicode_map,
    });
    let resources = document.add_object(dictionary! {"Font" => dictionary! {"F1" => type0}});
    let pages = document.new_object_id();
    let mut kids = Vec::new();
    let lines_per_page =
        ((PAGE_HEIGHT - 2.0 * MARGIN - FONT_SIZE) / LINE_HEIGHT).floor() as usize + 1;
    for chunk in lines.chunks(lines_per_page) {
        let mut operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), FONT_SIZE.into()]),
        ];
        for (index, line) in chunk.iter().enumerate() {
            let y = PAGE_HEIGHT - MARGIN - FONT_SIZE - index as f32 * LINE_HEIGHT;
            operations.push(Operation::new(
                "Tm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    MARGIN.into(),
                    y.into(),
                ],
            ));
            let encoded: Vec<u8> = line
                .chars()
                .flat_map(|ch| characters[&ch].0.to_be_bytes())
                .collect();
            operations.push(Operation::new(
                "Tj",
                vec![Object::String(encoded, lopdf::StringFormat::Hexadecimal)],
            ));
        }
        operations.push(Operation::new("ET", vec![]));
        let content = Content { operations }
            .encode()
            .map_err(|error| error.to_string())?;
        let stream = document.add_object(Stream::new(dictionary! {}, content));
        let page = document.add_object(dictionary! {"Type" => "Page", "Parent" => pages, "Contents" => stream, "Resources" => resources});
        kids.push(Object::Reference(page));
    }
    document.objects.insert(pages, dictionary! {"Type" => "Pages", "Count" => kids.len() as i64, "Kids" => kids, "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()]}.into());
    let catalog = document.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
    document.trailer.set("Root", catalog);
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_table_rows_and_pagination_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let headers = vec!["Name".into(), "Amount".into()];
        let rows = vec![
            serde_json::json!({"Name":"José", "Amount":42}),
            serde_json::json!(["Zoë", 7]),
        ];
        let content = format!(
            "Résumé — café €\n{}",
            "A long paragraph with words that should fit within the margins. ".repeat(500)
        );
        let filename = generate_pdf(
            &dir.path().to_path_buf(),
            "report",
            Some(&headers),
            Some(&rows),
            Some(&content),
        )
        .unwrap();
        let path = dir.path().join(filename);
        let pdf = Document::load(&path).unwrap();
        assert!(pdf.get_pages().len() > 1);
        let text = pdf_extract::extract_text(&path).unwrap();
        for expected in ["Résumé", "café", "€", "José", "42", "Zoë", "7"] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
    }

    #[test]
    fn invisible_control_bytes_do_not_panic() {
        let document = build_document(&find_system_font().unwrap(), &["a\0b\tc".into()]).unwrap();
        assert_eq!(document.get_pages().len(), 1);
    }

    #[test]
    fn empty_pdf_is_valid() {
        let document = build_document(&find_system_font().unwrap(), &[]).unwrap();
        assert_eq!(document.get_pages().len(), 1);
    }
}
