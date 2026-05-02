//! Row-by-row worksheet parse: avoids building a full `Vec<Vec<CellValue>>` in Rust for typical `<row>`-based sheets.

use crate::error::{Result, XlsxError};
use crate::xlsx::{
    attr_value, decode_cell_value, expand_dimension, local_name, parse_cell_ref, CellValue,
};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum StreamMode {
    Undecided,
    RowBased,
    Legacy,
}

pub struct WorksheetRowParser {
    reader: Reader<Cursor<Vec<u8>>>,
    buf: Vec<u8>,
    shared_strings: Vec<Arc<str>>,
    stream_mode: StreamMode,
    dim_max_row: Option<u32>,
    dim_max_col: Option<u32>,
    global_max_col: u32,
    in_sheet_data: bool,
    in_row: bool,
    current_row_r0: Option<u32>,
    row_cells: HashMap<u32, CellValue>,
    /// Next 0-based row index that must appear in output (for gap filling).
    next_emit_row0: u32,
    pending_rows: VecDeque<Vec<CellValue>>,
    xml_done: bool,
    legacy_cells: Option<HashMap<(u32, u32), CellValue>>,
    legacy_max_r: u32,
    legacy_max_c: u32,
    legacy_iter: Option<std::vec::IntoIter<Vec<CellValue>>>,
    pending_cell_ref: Option<String>,
    pending_cell_type: Option<String>,
    cell_value_written: bool,
    in_v: bool,
    in_is: bool,
    inline_t_depth: u32,
    text_buf: String,
}

impl WorksheetRowParser {
    pub fn new(bytes: Vec<u8>, shared_strings: Vec<Arc<str>>) -> Self {
        let mut reader = Reader::from_reader(Cursor::new(bytes));
        reader.config_mut().trim_text(true);
        Self {
            reader,
            buf: Vec::new(),
            shared_strings,
            stream_mode: StreamMode::Undecided,
            dim_max_row: None,
            dim_max_col: None,
            global_max_col: 0,
            in_sheet_data: false,
            in_row: false,
            current_row_r0: None,
            row_cells: HashMap::new(),
            next_emit_row0: 0,
            pending_rows: VecDeque::new(),
            xml_done: false,
            legacy_cells: None,
            legacy_max_r: 0,
            legacy_max_c: 0,
            legacy_iter: None,
            pending_cell_ref: None,
            pending_cell_type: None,
            cell_value_written: false,
            in_v: false,
            in_is: false,
            inline_t_depth: 0,
            text_buf: String::new(),
        }
    }

    fn ncols(&self) -> usize {
        let mc = self
            .dim_max_col
            .unwrap_or(self.global_max_col)
            .max(self.global_max_col);
        (mc + 1) as usize
    }

    fn switch_to_legacy(&mut self) {
        if self.stream_mode != StreamMode::Legacy {
            self.stream_mode = StreamMode::Legacy;
            self.legacy_cells = Some(HashMap::new());
            self.legacy_max_r = 0;
            self.legacy_max_c = 0;
        }
    }

    fn enqueue_row_dense(&mut self, r0: u32, cells: HashMap<u32, CellValue>) -> Result<()> {
        let ncols = self.ncols().max(1);
        while self.next_emit_row0 < r0 {
            self.pending_rows
                .push_back(vec![CellValue::Empty; ncols]);
            self.next_emit_row0 += 1;
        }
        let mut v = vec![CellValue::Empty; ncols];
        for (c, val) in cells {
            if let Some(slot) = v.get_mut(c as usize) {
                *slot = val;
            }
        }
        self.pending_rows.push_back(v);
        self.next_emit_row0 = r0.saturating_add(1);
        Ok(())
    }

    fn insert_legacy(&mut self, r: u32, c: u32, val: CellValue) {
        self.switch_to_legacy();
        let map = self.legacy_cells.as_mut().expect("legacy map");
        map.insert((r, c), val);
        self.legacy_max_r = self.legacy_max_r.max(r);
        self.legacy_max_c = self.legacy_max_c.max(c);
        self.global_max_col = self.global_max_col.max(c);
    }

    fn insert_row_cell(&mut self, r: u32, c: u32, val: CellValue) -> Result<()> {
        let row0 = self
            .current_row_r0
            .ok_or_else(|| XlsxError::InvalidWorkbook("cell outside row".into()))?;
        if r != row0 {
            return Err(XlsxError::InvalidWorkbook(format!(
                "cell row {r} does not match row tag {row0}"
            )));
        }
        self.row_cells.insert(c, val);
        self.global_max_col = self.global_max_col.max(c);
        Ok(())
    }

    fn build_legacy_grid(&mut self) -> Result<()> {
        if self.legacy_iter.is_some() {
            return Ok(());
        }
        let cells = self.legacy_cells.take().unwrap_or_default();
        if cells.is_empty() && self.legacy_max_r == 0 && self.legacy_max_c == 0 {
            self.legacy_iter = Some(Vec::new().into_iter());
            return Ok(());
        }
        let nrows = (self.legacy_max_r + 1) as usize;
        let ncols = (self.legacy_max_c + 1) as usize;
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
        self.legacy_iter = Some(grid.into_iter());
        Ok(())
    }

    fn tail_rowbased_sheetdata(&mut self) -> Result<()> {
        if self.stream_mode != StreamMode::RowBased {
            return Ok(());
        }
        if let Some(dm) = self.dim_max_row {
            let ncols = self.ncols().max(1);
            while self.next_emit_row0 <= dm {
                self.pending_rows
                    .push_back(vec![CellValue::Empty; ncols]);
                self.next_emit_row0 += 1;
            }
        }
        Ok(())
    }

    /// Parse forward until at least one output row is queued, legacy rows are ready, or XML is exhausted.
    fn pump(&mut self) -> Result<()> {
        if self.xml_done {
            return Ok(());
        }
        loop {
            if !self.pending_rows.is_empty() || self.legacy_iter.is_some() {
                return Ok(());
            }
            let ev = match self.reader.read_event_into(&mut self.buf) {
                Ok(e) => e,
                Err(e) => return Err(XlsxError::Xml(e.to_string())),
            };
            let ev = ev.into_owned();
            self.buf.clear();
            match ev {
                Event::Start(e) => {
                    let ln = local_name(e.name());
                    if ln == "dimension" {
                        if let Some(r) = attr_value(&e, "ref", &self.reader) {
                            if let Some((mr, mc)) = expand_dimension(&r)? {
                                self.dim_max_row = Some(self.dim_max_row.unwrap_or(mr).max(mr));
                                self.dim_max_col = Some(self.dim_max_col.unwrap_or(mc).max(mc));
                                self.global_max_col = self.global_max_col.max(mc);
                            }
                        }
                    } else if ln == "sheetData" {
                        self.in_sheet_data = true;
                    } else if self.in_sheet_data && ln == "row" {
                        if self.stream_mode == StreamMode::Undecided {
                            self.stream_mode = StreamMode::RowBased;
                        }
                        self.in_row = true;
                        self.row_cells.clear();
                        if let Some(rs) = attr_value(&e, "r", &self.reader) {
                            let r1: u32 = rs
                                .parse()
                                .map_err(|_| XlsxError::InvalidWorkbook(format!("bad row r={rs}")))?;
                            if r1 == 0 {
                                return Err(XlsxError::InvalidWorkbook("row r=0".into()));
                            }
                            self.current_row_r0 = Some(r1 - 1);
                        } else {
                            self.current_row_r0 = Some(self.next_emit_row0);
                        }
                    } else if self.in_sheet_data && ln == "c" {
                        if self.stream_mode == StreamMode::Undecided
                            || (self.stream_mode == StreamMode::RowBased && !self.in_row)
                        {
                            self.switch_to_legacy();
                        }
                        self.pending_cell_ref = attr_value(&e, "r", &self.reader);
                        self.pending_cell_type = attr_value(&e, "t", &self.reader);
                        self.cell_value_written = false;
                    } else if self.in_sheet_data && ln == "v" {
                        self.in_v = true;
                        self.text_buf.clear();
                    } else if self.in_sheet_data && ln == "is" {
                        self.in_is = true;
                        self.text_buf.clear();
                        self.inline_t_depth = 0;
                    } else if self.in_sheet_data && self.in_is && ln == "t" {
                        self.inline_t_depth += 1;
                    }
                }
                Event::Empty(e) => {
                    let ln = local_name(e.name());
                    if self.in_sheet_data && ln == "c" {
                        let cref = attr_value(&e, "r", &self.reader);
                        let ct = attr_value(&e, "t", &self.reader);
                        let (row, col) = if let Some(r) = cref {
                            parse_cell_ref(&r)?
                        } else {
                            continue;
                        };
                        self.global_max_col = self.global_max_col.max(col);
                        let val = CellValue::Empty;
                        match self.stream_mode {
                            StreamMode::Legacy => self.insert_legacy(row, col, val),
                            StreamMode::RowBased if self.in_row => {
                                self.insert_row_cell(row, col, val)?;
                            }
                            StreamMode::RowBased => {
                                self.switch_to_legacy();
                                self.insert_legacy(row, col, val);
                            }
                            StreamMode::Undecided => {
                                self.switch_to_legacy();
                                self.insert_legacy(row, col, val);
                            }
                        }
                        let _ = ct;
                    }
                }
                Event::Text(e) => {
                    let t = e
                        .unescape()
                        .map(|s| s.into_owned())
                        .unwrap_or_else(|_| String::new());
                    if self.in_v || (self.in_is && self.inline_t_depth > 0) {
                        self.text_buf.push_str(&t);
                    }
                }
                Event::End(e) => {
                    let ln = local_name(e.name());
                    if ln == "sheetData" {
                        if self.stream_mode == StreamMode::RowBased {
                            self.tail_rowbased_sheetdata()?;
                        }
                        if self.stream_mode == StreamMode::Legacy {
                            self.build_legacy_grid()?;
                        }
                        self.in_sheet_data = false;
                        self.xml_done = true;
                        return Ok(());
                    }
                    if !self.in_sheet_data {
                        continue;
                    }
                    if ln == "v" {
                        let cref = self
                            .pending_cell_ref
                            .clone()
                            .ok_or_else(|| XlsxError::InvalidWorkbook("cell <v> without ref".into()))?;
                        let (row, col) = parse_cell_ref(&cref)?;
                        let v = std::mem::take(&mut self.text_buf);
                        let val = decode_cell_value(
                            &self.pending_cell_type,
                            &v,
                            &self.shared_strings,
                        )?;
                        match self.stream_mode {
                            StreamMode::Legacy => self.insert_legacy(row, col, val),
                            StreamMode::RowBased if self.in_row => {
                                self.insert_row_cell(row, col, val)?;
                            }
                            StreamMode::RowBased => {
                                self.switch_to_legacy();
                                self.insert_legacy(row, col, val);
                            }
                            StreamMode::Undecided => {
                                self.switch_to_legacy();
                                self.insert_legacy(row, col, val);
                            }
                        }
                        self.cell_value_written = true;
                        self.in_v = false;
                    } else if ln == "t" && self.in_is && self.inline_t_depth > 0 {
                        self.inline_t_depth -= 1;
                    } else if ln == "is" {
                        let cref = self.pending_cell_ref.clone().ok_or_else(|| {
                            XlsxError::InvalidWorkbook("inlineStr without ref".into())
                        })?;
                        let (row, col) = parse_cell_ref(&cref)?;
                        let val = CellValue::Text(Arc::from(self.text_buf.as_str()));
                        self.text_buf.clear();
                        match self.stream_mode {
                            StreamMode::Legacy => self.insert_legacy(row, col, val),
                            StreamMode::RowBased if self.in_row => {
                                self.insert_row_cell(row, col, val)?;
                            }
                            StreamMode::RowBased => {
                                self.switch_to_legacy();
                                self.insert_legacy(row, col, val);
                            }
                            StreamMode::Undecided => {
                                self.switch_to_legacy();
                                self.insert_legacy(row, col, val);
                            }
                        }
                        self.cell_value_written = true;
                        self.in_is = false;
                        self.inline_t_depth = 0;
                        self.text_buf.clear();
                    } else if ln == "c" {
                        if let Some(cref) = self.pending_cell_ref.take() {
                            if !self.cell_value_written {
                                let (row, col) = parse_cell_ref(&cref)?;
                                let val = CellValue::Empty;
                                match self.stream_mode {
                                    StreamMode::Legacy => self.insert_legacy(row, col, val),
                                    StreamMode::RowBased if self.in_row => {
                                        self.insert_row_cell(row, col, val)?;
                                    }
                                    StreamMode::RowBased => {
                                        self.switch_to_legacy();
                                        self.insert_legacy(row, col, val);
                                    }
                                    StreamMode::Undecided => {
                                        self.switch_to_legacy();
                                        self.insert_legacy(row, col, val);
                                    }
                                }
                            }
                        }
                        self.pending_cell_type = None;
                        self.cell_value_written = false;
                    } else if ln == "row" {
                        self.in_row = false;
                        if self.stream_mode == StreamMode::RowBased {
                            let r0 = self
                                .current_row_r0
                                .ok_or_else(|| XlsxError::InvalidWorkbook("row end without r".into()))?;
                            let cells = std::mem::take(&mut self.row_cells);
                            self.enqueue_row_dense(r0, cells)?;
                            self.current_row_r0 = None;
                            if !self.pending_rows.is_empty() {
                                return Ok(());
                            }
                        }
                    }
                }
                Event::Eof => {
                    if self.stream_mode == StreamMode::Legacy && self.legacy_iter.is_none() {
                        self.build_legacy_grid()?;
                    }
                    if self.stream_mode == StreamMode::RowBased {
                        self.tail_rowbased_sheetdata()?;
                    }
                    self.xml_done = true;
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    /// Next data row in sheet order. Empty rows implied by `<dimension>` are yielded as all-empty vectors in row-based mode.
    pub fn next_row(&mut self) -> Result<Option<Vec<CellValue>>> {
        if let Some(ref mut it) = self.legacy_iter {
            return Ok(it.next());
        }
        if let Some(r) = self.pending_rows.pop_front() {
            return Ok(Some(r));
        }
        if self.xml_done {
            return Ok(None);
        }
        self.pump()?;
        if let Some(ref mut it) = self.legacy_iter {
            return Ok(it.next());
        }
        Ok(self.pending_rows.pop_front())
    }
}
