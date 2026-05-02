//! Streaming-friendly XLSX writer (OOXML package).

use crate::error::{Result, XlsxError};
use crate::xlsx::CellValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// 0-based column index → Excel column letters (A, B, …, Z, AA, …).
fn col_letters_zero_based(mut col: u32) -> String {
    let mut v = Vec::new();
    col += 1;
    while col > 0 {
        col -= 1;
        v.push((b'A' + (col % 26) as u8) as char);
        col /= 26;
    }
    v.into_iter().rev().collect()
}

fn cell_ref(row0: u32, col0: u32) -> String {
    format!("{}{}", col_letters_zero_based(col0), row0 + 1)
}

struct SharedStringTable {
    index: HashMap<String, u32>,
    strings: Vec<String>,
}

impl SharedStringTable {
    fn new() -> Self {
        Self {
            index: HashMap::new(),
            strings: Vec::new(),
        }
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.index.get(s) {
            return i;
        }
        let i = self.strings.len() as u32;
        self.index.insert(s.to_string(), i);
        self.strings.push(s.to_string());
        i
    }

    fn index_of(&self, s: &str) -> u32 {
        self.index[s]
    }

    fn build_from_grid(grid: &[Vec<CellValue>]) -> Self {
        let mut t = Self::new();
        for row in grid {
            for cell in row {
                if let CellValue::Text(a) = cell {
                    t.intern(a.as_ref());
                }
            }
        }
        t
    }
}

fn write_shared_strings_xml(w: &mut impl Write, sst: &SharedStringTable) -> Result<()> {
    let n = sst.strings.len();
    writeln!(
        w,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    )?;
    writeln!(
        w,
        r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{n}" uniqueCount="{n}">"#
    )?;
    for s in &sst.strings {
        writeln!(w, "<si><t>{}</t></si>", xml_escape(s))?;
    }
    writeln!(w, "</sst>")?;
    Ok(())
}

fn write_sheet_xml_with_sst(
    w: &mut impl Write,
    grid: &[Vec<CellValue>],
    sst: &SharedStringTable,
) -> Result<()> {
    let (nrows, ncols) = grid_dims(grid);
    let dim = if nrows == 0 || ncols == 0 {
        "A1".to_string()
    } else {
        format!(
            "{}:{}",
            cell_ref(0, 0),
            cell_ref(
                nrows.saturating_sub(1) as u32,
                ncols.saturating_sub(1) as u32,
            )
        )
    };

    writeln!(
        w,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    )?;
    writeln!(
        w,
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#
    )?;
    writeln!(w, r#"<dimension ref="{dim}"/>"#)?;
    writeln!(w, "<sheetData>")?;

    for (r, row) in grid.iter().enumerate() {
        let r1 = r + 1;
        writeln!(w, r#"<row r="{r1}">"#)?;
        for (c, cell) in row.iter().enumerate() {
            let cref = cell_ref(r as u32, c as u32);
            match cell {
                CellValue::Empty => {}
                CellValue::Bool(b) => {
                    let v = if *b { 1 } else { 0 };
                    writeln!(
                        w,
                        r#"<c r="{cref}" t="b"><v>{v}</v></c>"#
                    )?;
                }
                CellValue::Number(n) => {
                    writeln!(w, r#"<c r="{cref}"><v>{n}</v></c>"#)?;
                }
                CellValue::Text(a) => {
                    let idx = sst.index_of(a.as_ref());
                    writeln!(
                        w,
                        r#"<c r="{cref}" t="s"><v>{idx}</v></c>"#
                    )?;
                }
            }
        }
        writeln!(w, "</row>")?;
    }

    writeln!(w, "</sheetData></worksheet>")?;
    Ok(())
}

fn grid_dims(grid: &[Vec<CellValue>]) -> (usize, usize) {
    let nrows = grid.len();
    let ncols = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    (nrows, ncols)
}

/// Write a single-sheet workbook with shared-string deduplication (memory ∝ unique strings, not cells).
pub fn write_xlsx(path: &Path, sheet_name: &str, grid: &[Vec<CellValue>]) -> Result<()> {
    let sst = SharedStringTable::build_from_grid(grid);

    let file = File::create(path).map_err(XlsxError::Io)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts)?;
    zip.write_all(content_types.as_bytes())?;

    let root_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;
    zip.start_file("_rels/.rels", opts)?;
    zip.write_all(root_rels.as_bytes())?;

    let core = r#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator>fast-xlsx</dc:creator></cp:coreProperties>"#;
    zip.start_file("docProps/core.xml", opts)?;
    zip.write_all(core.as_bytes())?;

    let app = r#"<?xml version="1.0" encoding="UTF-8"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>fast-xlsx</Application></Properties>"#;
    zip.start_file("docProps/app.xml", opts)?;
    zip.write_all(app.as_bytes())?;

    let escaped_name = xml_escape(sheet_name);
    let workbook = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="{escaped_name}" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#
    );
    zip.start_file("xl/workbook.xml", opts)?;
    zip.write_all(workbook.as_bytes())?;

    let wb_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#;
    zip.start_file("xl/_rels/workbook.xml.rels", opts)?;
    zip.write_all(wb_rels.as_bytes())?;

    let styles = r#"<?xml version="1.0" encoding="UTF-8"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="1"><font><sz val="11"/><color theme="1"/><name val="Calibri"/><family val="2"/></font></fonts>
<fills count="1"><fill><patternFill patternType="none"/></fill></fills>
<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs>
</styleSheet>"#;
    zip.start_file("xl/styles.xml", opts)?;
    zip.write_all(styles.as_bytes())?;

    zip.start_file("xl/sharedStrings.xml", opts)?;
    let mut sst_buf: Vec<u8> = Vec::new();
    write_shared_strings_xml(&mut sst_buf, &sst)?;
    zip.write_all(&sst_buf)?;

    zip.start_file("xl/worksheets/sheet1.xml", opts)?;
    let mut sheet_buf: Vec<u8> = Vec::new();
    write_sheet_xml_with_sst(&mut sheet_buf, grid, &sst)?;
    zip.write_all(&sheet_buf)?;

    zip.finish().map_err(XlsxError::Zip)?;
    Ok(())
}

/// Constant-memory row writer: one worksheet, inline strings (no sharedStrings.xml).
pub struct StreamingWriter {
    zip: Option<ZipWriter<File>>,
    row_1based: u32,
    in_sheet: bool,
}

impl StreamingWriter {
    pub fn create(path: &Path, sheet_name: &str) -> Result<Self> {
        let file = File::create(path).map_err(XlsxError::Io)?;
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
</Types>"#;
        zip.start_file("[Content_Types].xml", opts)?;
        zip.write_all(content_types.as_bytes())?;

        let root_rels = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#;
        zip.start_file("_rels/.rels", opts)?;
        zip.write_all(root_rels.as_bytes())?;

        let core = r#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator>fast-xlsx</dc:creator></cp:coreProperties>"#;
        zip.start_file("docProps/core.xml", opts)?;
        zip.write_all(core.as_bytes())?;

        let app = r#"<?xml version="1.0" encoding="UTF-8"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>fast-xlsx</Application></Properties>"#;
        zip.start_file("docProps/app.xml", opts)?;
        zip.write_all(app.as_bytes())?;

        let escaped_name = xml_escape(sheet_name);
        let workbook = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="{escaped_name}" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#
        );
        zip.start_file("xl/workbook.xml", opts)?;
        zip.write_all(workbook.as_bytes())?;

        let wb_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#;
        zip.start_file("xl/_rels/workbook.xml.rels", opts)?;
        zip.write_all(wb_rels.as_bytes())?;

        let styles = r#"<?xml version="1.0" encoding="UTF-8"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="1"><font><sz val="11"/><color theme="1"/><name val="Calibri"/><family val="2"/></font></fonts>
<fills count="1"><fill><patternFill patternType="none"/></fill></fills>
<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs>
</styleSheet>"#;
        zip.start_file("xl/styles.xml", opts)?;
        zip.write_all(styles.as_bytes())?;

        zip.start_file("xl/worksheets/sheet1.xml", opts)?;
        let header = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheetData>"#;
        zip.write_all(header.as_bytes())?;

        Ok(Self {
            zip: Some(zip),
            row_1based: 0,
            in_sheet: true,
        })
    }

    pub fn write_row(&mut self, row: &[CellValue]) -> Result<()> {
        if !self.in_sheet {
            return Err(XlsxError::InvalidWorkbook(
                "streaming writer already finished".into(),
            ));
        }
        let zip = self
            .zip
            .as_mut()
            .ok_or_else(|| XlsxError::InvalidWorkbook("zip closed".into()))?;
        self.row_1based += 1;
        let r = self.row_1based;

        use std::fmt::Write;
        let mut line = String::with_capacity(256);
        write!(&mut line, r#"<row r="{}">"#, r).unwrap();
        for (c0, cell) in row.iter().enumerate() {
            let cref = cell_ref(r - 1, c0 as u32);
            match cell {
                CellValue::Empty => {}
                CellValue::Bool(b) => {
                    let v = if *b { 1 } else { 0 };
                    write!(&mut line, r#"<c r="{}" t="b"><v>{}</v></c>"#, cref, v).unwrap();
                }
                CellValue::Number(n) => {
                    write!(&mut line, r#"<c r="{}"><v>{}</v></c>"#, cref, n).unwrap();
                }
                CellValue::Text(a) => {
                    let t = xml_escape(a.as_ref());
                    write!(
                        &mut line,
                        r#"<c r="{}" t="inlineStr"><is><t>{}</t></is></c>"#,
                        cref, t
                    )
                    .unwrap();
                }
            }
        }
        line.push_str("</row>\n");
        zip.write_all(line.as_bytes()).map_err(XlsxError::Io)?;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        if !self.in_sheet {
            return Ok(());
        }
        if let Some(mut z) = self.zip.take() {
            if let Err(e) = z.write_all(b"</sheetData></worksheet>") {
                self.zip = Some(z);
                return Err(XlsxError::Io(e));
            }
            z.finish().map_err(XlsxError::Zip)?;
        }
        self.in_sheet = false;
        Ok(())
    }
}

impl Drop for StreamingWriter {
    fn drop(&mut self) {
        if self.in_sheet {
            self.in_sheet = false;
            if let Some(mut z) = self.zip.take() {
                let _ = z.write_all(b"</sheetData></worksheet>");
                let _ = z.finish();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xlsx;
    use std::sync::Arc;

    fn text(s: &str) -> CellValue {
        CellValue::Text(Arc::from(s))
    }

    #[test]
    fn roundtrip_write_read() {
        let grid = vec![
            vec![text("a"), CellValue::Number(1.5), CellValue::Bool(true)],
            vec![CellValue::Empty, text("a"), CellValue::Empty],
        ];
        let path = std::env::temp_dir().join("fast_xlsx_wtest.xlsx");
        write_xlsx(&path, "S", &grid).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let wb = xlsx::parse_workbook(bytes).unwrap();
        let out = xlsx::read_sheet_grid(&wb, 0).unwrap();
        assert_eq!(out.len(), grid.len());
        assert_eq!(out[0].len(), grid[0].len());
        match (&out[0][0], &grid[0][0]) {
            (CellValue::Text(a), CellValue::Text(b)) => assert_eq!(a.as_ref(), b.as_ref()),
            _ => panic!("expected text"),
        }
    }
}
