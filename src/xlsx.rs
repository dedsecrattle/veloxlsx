//! Minimal OOXML reader: shared strings, workbook sheets, one worksheet grid.

use crate::error::{Result, XlsxError};
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::{BufRead, Cursor, Read};
use std::sync::Arc;
use std::sync::Mutex;
use zip::ZipArchive;

#[derive(Clone, Debug)]
pub struct SheetInfo {
    pub name: String,
    /// Path inside the ZIP, e.g. `xl/worksheets/sheet1.xml`
    pub path: String,
}

#[derive(Clone, Debug)]
pub enum CellValue {
    Empty,
    Bool(bool),
    Number(f64),
    Text(Arc<str>),
}

pub struct WorkbookData {
    archive: Mutex<ZipArchive<Cursor<Vec<u8>>>>,
    pub sheets: Vec<SheetInfo>,
    pub shared_strings: Vec<Arc<str>>,
}

pub fn read_file_bytes(path: &std::path::Path) -> Result<Vec<u8>> {
    Ok(std::fs::read(path)?)
}

/// Parses the workbook and keeps a single ZIP archive open for subsequent sheet reads.
pub fn parse_workbook(zip_bytes: Vec<u8>) -> Result<WorkbookData> {
    let cursor = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let rels = read_workbook_rels(&mut archive)?;
    let (sheet_infos, _defined_names_skipped) = read_workbook_xml(&mut archive, &rels)?;
    let shared_strings = read_shared_strings(&mut archive)?;
    Ok(WorkbookData {
        archive: Mutex::new(archive),
        sheets: sheet_infos,
        shared_strings,
    })
}

pub fn read_sheet_grid(data: &WorkbookData, sheet_index: usize) -> Result<Vec<Vec<CellValue>>> {
    let info = data
        .sheets
        .get(sheet_index)
        .ok_or_else(|| XlsxError::InvalidWorkbook("sheet index out of range".into()))?;
    read_sheet_grid_inner(data, info.path.as_str())
}

pub fn read_sheet_grid_by_name(data: &WorkbookData, name: &str) -> Result<Vec<Vec<CellValue>>> {
    let idx = data
        .sheets
        .iter()
        .position(|s| s.name == name)
        .ok_or_else(|| XlsxError::SheetNotFound(name.to_string()))?;
    read_sheet_grid(data, idx)
}

fn read_sheet_grid_inner(data: &WorkbookData, worksheet_path: &str) -> Result<Vec<Vec<CellValue>>> {
    let mut guard = data
        .archive
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let file = guard
        .by_name(worksheet_path)
        .map_err(|_| XlsxError::MissingEntry(worksheet_path.to_string()))?;
    let reader = std::io::BufReader::new(file);
    parse_worksheet(reader, &data.shared_strings)
}

/// Inflated worksheet XML bytes (for resumable row streaming without holding the ZIP open).
pub(crate) fn read_worksheet_inflated(data: &WorkbookData, worksheet_path: &str) -> Result<Vec<u8>> {
    let mut guard = data
        .archive
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut file = guard
        .by_name(worksheet_path)
        .map_err(|_| XlsxError::MissingEntry(worksheet_path.to_string()))?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buf)?;
    Ok(buf)
}

fn read_zip_entry_to_string(archive: &mut ZipArchive<Cursor<Vec<u8>>>, path: &str) -> Result<String> {
    let mut file = archive
        .by_name(path)
        .map_err(|_| XlsxError::MissingEntry(path.to_string()))?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    Ok(s)
}

fn read_workbook_rels(archive: &mut ZipArchive<Cursor<Vec<u8>>>) -> Result<HashMap<String, String>> {
    let xml = read_zip_entry_to_string(archive, "xl/_rels/workbook.xml.rels")?;
    parse_relationships(&xml)
}

fn parse_relationships(xml: &str) -> Result<HashMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut map = HashMap::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e) | Event::Start(ref e)) => {
                if local_name(e.name()) == "Relationship" {
                    let mut id = None;
                    let mut target = None;
                    for a in e.attributes().flatten() {
                        let k = reader.decoder().decode(a.key.as_ref()).unwrap_or_default();
                        if k == "Id" {
                            id = Some(String::from_utf8_lossy(a.value.as_ref()).trim().to_string());
                        } else if k == "Target" {
                            let t = String::from_utf8_lossy(a.value.as_ref()).trim().to_string();
                            target = Some(normalize_workbook_target(&t));
                        }
                    }
                    if let (Some(id), Some(t)) = (id, target) {
                        map.insert(id, t);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(map)
}

/// Turn `worksheets/sheet1.xml` or `/xl/worksheets/sheet1.xml` into `xl/worksheets/sheet1.xml`
fn normalize_workbook_target(target: &str) -> String {
    let t = target.replace('\\', "/");
    if let Some(rest) = t.strip_prefix("/xl/") {
        format!("xl/{rest}")
    } else if let Some(rest) = t.strip_prefix("xl/") {
        format!("xl/{rest}")
    } else {
        format!("xl/{}", t.trim_start_matches('/'))
    }
}

fn push_sheet_from_element(
    e: &quick_xml::events::BytesStart<'_>,
    reader: &Reader<&[u8]>,
    rels: &HashMap<String, String>,
    sheets: &mut Vec<SheetInfo>,
) -> Result<()> {
    let mut name = None;
    let mut rid = None;
    for a in e.attributes().flatten() {
        let k = reader.decoder().decode(a.key.as_ref()).unwrap_or_default();
        if k == "name" {
            name = Some(String::from_utf8_lossy(a.value.as_ref()).trim().to_string());
        } else if k == "r:id" || k.ends_with("}id") {
            rid = Some(String::from_utf8_lossy(a.value.as_ref()).trim().to_string());
        }
    }
    if let (Some(name), Some(rid)) = (name, rid) {
        let path = rels
            .get(&rid)
            .cloned()
            .ok_or_else(|| XlsxError::InvalidWorkbook(format!("missing rel {rid}")))?;
        sheets.push(SheetInfo { name, path });
    }
    Ok(())
}

fn read_workbook_xml(
    archive: &mut ZipArchive<Cursor<Vec<u8>>>,
    rels: &HashMap<String, String>,
) -> Result<(Vec<SheetInfo>, ())> {
    let xml = read_zip_entry_to_string(archive, "xl/workbook.xml")?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut sheets = Vec::new();
    let mut in_sheets = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if local_name(e.name()) == "sheets" {
                    in_sheets = true;
                } else if in_sheets && local_name(e.name()) == "sheet" {
                    push_sheet_from_element(e, &reader, rels, &mut sheets)?;
                }
            }
            Ok(Event::Empty(ref e)) => {
                if in_sheets && local_name(e.name()) == "sheet" {
                    push_sheet_from_element(e, &reader, rels, &mut sheets)?;
                }
            }
            Ok(Event::End(ref e)) => {
                if local_name(e.name()) == "sheets" {
                    in_sheets = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    if sheets.is_empty() {
        return Err(XlsxError::InvalidWorkbook("no sheets in workbook".into()));
    }
    Ok((sheets, ()))
}

fn read_shared_strings(archive: &mut ZipArchive<Cursor<Vec<u8>>>) -> Result<Vec<Arc<str>>> {
    let xml = match read_zip_entry_to_string(archive, "xl/sharedStrings.xml") {
        Ok(s) => s,
        Err(XlsxError::MissingEntry(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    parse_shared_strings(&xml)
}

fn parse_shared_strings(xml: &str) -> Result<Vec<Arc<str>>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut in_si = false;
    let mut t_depth: u32 = 0;
    let mut current = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let ln = local_name(e.name());
                if ln == "si" {
                    in_si = true;
                    current.clear();
                    t_depth = 0;
                } else if in_si && ln == "t" {
                    t_depth += 1;
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_si && t_depth > 0 {
                    current.push_str(&reader.decoder().decode(e.as_ref()).unwrap_or_default());
                }
            }
            Ok(Event::End(ref e)) => {
                let ln = local_name(e.name());
                if ln == "t" && in_si && t_depth > 0 {
                    t_depth -= 1;
                } else if ln == "si" {
                    out.push(Arc::from(current.as_str()));
                    in_si = false;
                    t_depth = 0;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

fn parse_worksheet<R: BufRead>(input: R, shared_strings: &[Arc<str>]) -> Result<Vec<Vec<CellValue>>> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut max_row: u32 = 0;
    let mut max_col: u32 = 0;
    let mut cells: HashMap<(u32, u32), CellValue> = HashMap::new();

    let mut in_sheet_data = false;
    let mut pending_cell_ref: Option<String> = None;
    let mut pending_cell_type: Option<String> = None;
    let mut cell_value_written = false;
    let mut in_v = false;
    let mut in_is = false;
    let mut inline_t_depth: u32 = 0;
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let ln = local_name(e.name());
                if ln == "dimension" {
                    if let Some(r) = attr_value(e, "ref", &reader) {
                        if let Some((mr, mc)) = expand_dimension(&r)? {
                            max_row = max_row.max(mr);
                            max_col = max_col.max(mc);
                        }
                    }
                } else if ln == "sheetData" {
                    in_sheet_data = true;
                } else if in_sheet_data && ln == "c" {
                    pending_cell_ref = attr_value(e, "r", &reader);
                    pending_cell_type = attr_value(e, "t", &reader);
                    cell_value_written = false;
                } else if in_sheet_data && ln == "v" {
                    in_v = true;
                    text_buf.clear();
                } else if in_sheet_data && ln == "is" {
                    in_is = true;
                    text_buf.clear();
                    inline_t_depth = 0;
                } else if in_sheet_data && in_is && ln == "t" {
                    inline_t_depth += 1;
                }
            }
            Ok(Event::Empty(ref e)) => {
                let ln = local_name(e.name());
                if in_sheet_data && ln == "c" {
                    let cref = attr_value(e, "r", &reader);
                    let ct = attr_value(e, "t", &reader);
                    let (row, col) = if let Some(r) = cref {
                        parse_cell_ref(&r)?
                    } else {
                        continue;
                    };
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                    cells.insert((row, col), CellValue::Empty);
                    let _ = ct;
                }
            }
            Ok(Event::Text(ref e)) => {
                let t = reader.decoder().decode(e.as_ref()).unwrap_or_default();
                if in_v || (in_is && inline_t_depth > 0) {
                    text_buf.push_str(&t);
                }
            }
            Ok(Event::End(ref e)) => {
                let ln = local_name(e.name());
                if ln == "sheetData" {
                    in_sheet_data = false;
                } else if in_sheet_data && ln == "v" {
                    let cref = pending_cell_ref
                        .clone()
                        .ok_or_else(|| XlsxError::InvalidWorkbook("cell <v> without ref".into()))?;
                    let (row, col) = parse_cell_ref(&cref)?;
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                    let v = std::mem::take(&mut text_buf);
                    let val = decode_cell_value(&pending_cell_type, &v, shared_strings)?;
                    cells.insert((row, col), val);
                    cell_value_written = true;
                    in_v = false;
                } else if in_sheet_data && ln == "t" && in_is && inline_t_depth > 0 {
                    inline_t_depth -= 1;
                } else if in_sheet_data && ln == "is" {
                    let cref = pending_cell_ref.clone().ok_or_else(|| {
                        XlsxError::InvalidWorkbook("inlineStr without ref".into())
                    })?;
                    let (row, col) = parse_cell_ref(&cref)?;
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                    let val = CellValue::Text(Arc::from(text_buf.as_str()));
                    text_buf.clear();
                    cells.insert((row, col), val);
                    cell_value_written = true;
                    in_is = false;
                    inline_t_depth = 0;
                    text_buf.clear();
                } else if in_sheet_data && ln == "c" {
                    if let Some(cref) = pending_cell_ref.take() {
                        if !cell_value_written {
                            let (row, col) = parse_cell_ref(&cref)?;
                            max_row = max_row.max(row);
                            max_col = max_col.max(col);
                            cells.insert((row, col), CellValue::Empty);
                        }
                    }
                    pending_cell_type = None;
                    cell_value_written = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XlsxError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let nrows = (max_row + 1) as usize;
    let ncols = (max_col + 1) as usize;
    let mut grid: Vec<Vec<CellValue>> = (0..nrows)
        .map(|_| (0..ncols).map(|_| CellValue::Empty).collect())
        .collect();
    for ((r, c), v) in cells {
        if let Some(row) = grid.get_mut(r as usize) {
            if let Some(cell) = row.get_mut(c as usize) {
                *cell = v;
            }
        }
    }
    Ok(grid)
}

pub(crate) fn expand_dimension(dim: &str) -> Result<Option<(u32, u32)>> {
    let dim = dim.trim();
    if dim.is_empty() {
        return Ok(None);
    }
    let parts: Vec<&str> = dim.split(':').collect();
    let end = parts.last().copied().unwrap_or(dim);
    let (r, c) = parse_cell_ref(end)?;
    Ok(Some((r, c)))
}

pub(crate) fn decode_cell_value(
    cell_type: &Option<String>,
    raw: &str,
    shared_strings: &[Arc<str>],
) -> Result<CellValue> {
    let trimmed = raw.trim();
    match cell_type.as_deref() {
        Some("s") => {
            let idx: usize = trimmed
                .parse()
                .map_err(|_| XlsxError::InvalidNumber(trimmed.to_string()))?;
            let s = shared_strings
                .get(idx)
                .map(Arc::clone)
                .unwrap_or_else(|| Arc::from(""));
            Ok(CellValue::Text(s))
        }
        Some("inlineStr") => Ok(CellValue::Text(Arc::from(trimmed))),
        Some("b") => Ok(CellValue::Bool(
            trimmed != "0" && !trimmed.eq_ignore_ascii_case("false"),
        )),
        Some("e") => Ok(CellValue::Text(Arc::from(format!("#ERROR:{trimmed}").into_boxed_str()))),
        Some("str") | Some("n") | None => {
            if trimmed.is_empty() {
                Ok(CellValue::Empty)
            } else if let Ok(n) = trimmed.parse::<f64>() {
                Ok(CellValue::Number(n))
            } else {
                Ok(CellValue::Text(Arc::from(trimmed)))
            }
        }
        Some(other) => {
            if let Ok(n) = trimmed.parse::<f64>() {
                Ok(CellValue::Number(n))
            } else {
                Ok(CellValue::Text(Arc::from(
                    format!("[{other}]{trimmed}").into_boxed_str(),
                )))
            }
        }
    }
}

pub(crate) fn parse_cell_ref(r: &str) -> Result<(u32, u32)> {
    let r = r.trim();
    let mut col_part = String::new();
    let mut row_part = String::new();
    for ch in r.chars() {
        if ch.is_ascii_alphabetic() {
            if !row_part.is_empty() {
                return Err(XlsxError::InvalidCellRef(r.to_string()));
            }
            col_part.push(ch);
        } else if ch.is_ascii_digit() {
            row_part.push(ch);
        }
    }
    if col_part.is_empty() || row_part.is_empty() {
        return Err(XlsxError::InvalidCellRef(r.to_string()));
    }
    let col = col_letters_to_zero_based(&col_part)?;
    let row: u32 = row_part
        .parse::<u32>()
        .map_err(|_| XlsxError::InvalidCellRef(r.to_string()))?;
    if row == 0 {
        return Err(XlsxError::InvalidCellRef(r.to_string()));
    }
    Ok((row - 1, col))
}

pub(crate) fn col_letters_to_zero_based(letters: &str) -> Result<u32> {
    let mut col: u32 = 0;
    for b in letters.bytes() {
        let c = b.to_ascii_uppercase();
        if !c.is_ascii_uppercase() {
            return Err(XlsxError::InvalidCellRef(letters.to_string()));
        }
        col = col
            .checked_mul(26)
            .and_then(|acc| acc.checked_add((c - b'A' + 1) as u32))
            .ok_or_else(|| XlsxError::InvalidCellRef(letters.to_string()))?;
    }
    Ok(col - 1)
}

pub(crate) fn attr_value(
    e: &quick_xml::events::BytesStart<'_>,
    key: &str,
    reader: &Reader<impl std::io::BufRead>,
) -> Option<String> {
    for a in e.attributes().flatten() {
        let k = reader.decoder().decode(a.key.as_ref()).unwrap_or_default();
        if k == key {
            return Some(String::from_utf8_lossy(a.value.as_ref()).trim().to_string());
        }
    }
    None
}

pub(crate) fn local_name(name: QName) -> String {
    let raw = String::from_utf8_lossy(name.as_ref());
    if let Some(i) = raw.rfind('}') {
        raw[i + 1..].to_string()
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_letters() {
        assert_eq!(col_letters_to_zero_based("A").unwrap(), 0);
        assert_eq!(col_letters_to_zero_based("Z").unwrap(), 25);
        assert_eq!(col_letters_to_zero_based("AA").unwrap(), 26);
    }

    #[test]
    fn cell_ref() {
        assert_eq!(parse_cell_ref("A1").unwrap(), (0, 0));
        assert_eq!(parse_cell_ref("B2").unwrap(), (1, 1));
    }
}
