mod error;
mod row_stream;
mod write;
mod xlsx;

use error::XlsxError;
use pyo3::exceptions::{PyStopIteration, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyBool, PyList, PyModule, PyString};
use std::path::PathBuf;
use std::sync::Arc;

#[pyclass(name = "Workbook")]
struct PyWorkbook {
    inner: Arc<xlsx::WorkbookData>,
}

#[pyclass(name = "Sheet")]
struct PySheet {
    workbook: Arc<xlsx::WorkbookData>,
    index: usize,
    name: String,
}

#[pyclass(name = "StreamWriter")]
struct PyStreamWriter {
    inner: Option<write::StreamingWriter>,
}

#[pyclass(name = "RowIter")]
struct PyRowIter {
    parser: Option<row_stream::WorksheetRowParser>,
}

#[pymethods]
impl PyWorkbook {
    #[getter]
    fn sheet_names(&self) -> Vec<String> {
        self.inner.sheets.iter().map(|s| s.name.clone()).collect()
    }

    fn __len__(&self) -> usize {
        self.inner.sheets.len()
    }

    fn __getitem__(&self, py: Python<'_>, key: Bound<'_, PyAny>) -> PyResult<Py<PySheet>> {
        let index = if let Ok(i) = key.extract::<isize>() {
            let n = self.inner.sheets.len() as isize;
            let idx = if i < 0 { n + i } else { i };
            if idx < 0 || idx >= n {
                return Err(PyValueError::new_err("sheet index out of range"));
            }
            idx as usize
        } else if let Ok(name) = key.extract::<String>() {
            self.inner
                .sheets
                .iter()
                .position(|s| s.name == name)
                .ok_or_else(|| PyValueError::new_err(format!("sheet not found: {name}")))?
        } else {
            return Err(PyValueError::new_err("sheet key must be int or str"));
        };
        let name = self.inner.sheets[index].name.clone();
        Py::new(
            py,
            PySheet {
                workbook: Arc::clone(&self.inner),
                index,
                name,
            },
        )
    }

    fn read_sheet(&self, py: Python<'_>, sheet: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let grid = if let Ok(i) = sheet.extract::<isize>() {
            let n = self.inner.sheets.len() as isize;
            let idx = if i < 0 { n + i } else { i };
            if idx < 0 || idx >= n {
                return Err(PyValueError::new_err("sheet index out of range"));
            }
            xlsx::read_sheet_grid(&self.inner, idx as usize).map_err(py_err)?
        } else if let Ok(name) = sheet.extract::<String>() {
            xlsx::read_sheet_grid_by_name(&self.inner, &name).map_err(py_err)?
        } else {
            return Err(PyValueError::new_err("sheet must be int or str"));
        };
        grid_to_py(py, &grid)
    }

    fn iter_rows(&self, py: Python<'_>, sheet: Bound<'_, PyAny>) -> PyResult<Py<PyRowIter>> {
        let index = if let Ok(i) = sheet.extract::<isize>() {
            let n = self.inner.sheets.len() as isize;
            let idx = if i < 0 { n + i } else { i };
            if idx < 0 || idx >= n {
                return Err(PyValueError::new_err("sheet index out of range"));
            }
            idx as usize
        } else if let Ok(name) = sheet.extract::<String>() {
            self.inner
                .sheets
                .iter()
                .position(|s| s.name == name)
                .ok_or_else(|| PyValueError::new_err(format!("sheet not found: {name}")))?
        } else {
            return Err(PyValueError::new_err("sheet must be int or str"));
        };
        let bytes = xlsx::read_worksheet_inflated(&self.inner, &self.inner.sheets[index].path)
            .map_err(py_err)?;
        let parser = row_stream::WorksheetRowParser::new(bytes, self.inner.shared_strings.clone());
        Py::new(py, PyRowIter {
            parser: Some(parser),
        })
    }
}

#[pymethods]
impl PySheet {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    fn to_list(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let grid = xlsx::read_sheet_grid(&self.workbook, self.index).map_err(py_err)?;
        grid_to_py(py, &grid)
    }

    fn iter_rows(&self, py: Python<'_>) -> PyResult<Py<PyRowIter>> {
        let bytes =
            xlsx::read_worksheet_inflated(&self.workbook, &self.workbook.sheets[self.index].path)
                .map_err(py_err)?;
        let parser =
            row_stream::WorksheetRowParser::new(bytes, self.workbook.shared_strings.clone());
        Py::new(py, PyRowIter {
            parser: Some(parser),
        })
    }
}

#[pymethods]
impl PyRowIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let Some(p) = self.parser.as_mut() else {
            return Err(PyStopIteration::new_err(()));
        };
        match p.next_row().map_err(py_err)? {
            None => {
                self.parser = None;
                Err(PyStopIteration::new_err(()))
            }
            Some(row) => row_to_py(py, &row),
        }
    }
}

fn row_to_py(py: Python<'_>, row: &[xlsx::CellValue]) -> PyResult<Py<PyAny>> {
    let py_row = PyList::empty(py);
    for cell in row {
        py_row.append(cell_value_to_py(py, cell)?)?;
    }
    Ok(py_row.into_any().unbind())
}

#[pymethods]
impl PyStreamWriter {
    #[new]
    #[pyo3(signature = (path, sheet_name=None))]
    fn new(path: PathBuf, sheet_name: Option<String>) -> PyResult<Self> {
        let name = sheet_name.unwrap_or_else(|| "Sheet1".to_string());
        let w = write::StreamingWriter::create(&path, &name).map_err(py_err)?;
        Ok(Self { inner: Some(w) })
    }

    fn write_row(&mut self, py: Python<'_>, row: Bound<'_, PyAny>) -> PyResult<()> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("StreamWriter already closed"))?;
        let cells = py_row_to_vec(py, &row)?;
        inner.write_row(&cells).map_err(py_err)
    }

    fn close(&mut self) -> PyResult<()> {
        if let Some(mut w) = self.inner.take() {
            w.finish().map_err(py_err)?;
        }
        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        mut slf: PyRefMut<'_, Self>,
        _exc_type: Bound<'_, PyAny>,
        _exc: Bound<'_, PyAny>,
        _tb: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        slf.close()
    }
}

impl Drop for PyStreamWriter {
    fn drop(&mut self) {
        if let Some(mut w) = self.inner.take() {
            let _ = w.finish();
        }
    }
}

fn grid_to_py(py: Python<'_>, grid: &[Vec<xlsx::CellValue>]) -> PyResult<Py<PyAny>> {
    let rows = PyList::empty(py);
    for row in grid {
        let py_row = PyList::empty(py);
        for cell in row {
            py_row.append(cell_value_to_py(py, cell)?)?;
        }
        rows.append(py_row)?;
    }
    Ok(rows.into_any().unbind())
}

fn cell_value_to_py(py: Python<'_>, v: &xlsx::CellValue) -> PyResult<Py<PyAny>> {
    match v {
        xlsx::CellValue::Empty => Ok(py.None()),
        xlsx::CellValue::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        xlsx::CellValue::Number(n) => Ok(n.into_pyobject(py)?.to_owned().into_any().unbind()),
        xlsx::CellValue::Text(s) => {
            Ok(PyString::new(py, s.as_ref()).into_any().unbind())
        }
    }
}

fn py_row_to_vec(py: Python<'_>, row: &Bound<'_, PyAny>) -> PyResult<Vec<xlsx::CellValue>> {
    let list = row.downcast::<PyList>()?;
    let mut r = Vec::with_capacity(list.len());
    for cell in list.iter() {
        r.push(py_to_cell(py, &cell)?);
    }
    Ok(r)
}

fn py_to_grid(py: Python<'_>, rows: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<xlsx::CellValue>>> {
    let list = rows.downcast::<PyList>()?;
    let mut out = Vec::with_capacity(list.len());
    for item in list.iter() {
        out.push(py_row_to_vec(py, &item)?);
    }
    Ok(out)
}

fn py_to_cell(_py: Python<'_>, cell: &Bound<'_, PyAny>) -> PyResult<xlsx::CellValue> {
    if cell.is_none() {
        return Ok(xlsx::CellValue::Empty);
    }
    if cell.is_instance_of::<PyBool>() {
        return Ok(xlsx::CellValue::Bool(cell.extract()?));
    }
    if let Ok(i) = cell.extract::<i64>() {
        return Ok(xlsx::CellValue::Number(i as f64));
    }
    if let Ok(f) = cell.extract::<f64>() {
        return Ok(xlsx::CellValue::Number(f));
    }
    if let Ok(s) = cell.extract::<String>() {
        return Ok(xlsx::CellValue::Text(Arc::from(s.into_boxed_str())));
    }
    Err(PyValueError::new_err(
        "cell must be None, bool, int, float, or str",
    ))
}

fn py_err(e: XlsxError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

#[pyfunction]
#[pyo3(signature = (path, sheet=None))]
fn read_xlsx(
    py: Python<'_>,
    path: PathBuf,
    sheet: Option<Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let bytes = xlsx::read_file_bytes(&path).map_err(py_err)?;
    let wb = xlsx::parse_workbook(bytes).map_err(py_err)?;
    let grid = match sheet {
        None => xlsx::read_sheet_grid(&wb, 0).map_err(py_err)?,
        Some(s) => {
            if let Ok(i) = s.extract::<isize>() {
                let n = wb.sheets.len() as isize;
                let idx = if i < 0 { n + i } else { i };
                if idx < 0 || idx >= n {
                    return Err(PyValueError::new_err("sheet index out of range"));
                }
                xlsx::read_sheet_grid(&wb, idx as usize).map_err(py_err)?
            } else if let Ok(name) = s.extract::<String>() {
                xlsx::read_sheet_grid_by_name(&wb, &name).map_err(py_err)?
            } else {
                return Err(PyValueError::new_err("sheet must be int or str or None"));
            }
        }
    };
    grid_to_py(py, &grid)
}

#[pyfunction]
#[pyo3(signature = (path, rows, sheet=None))]
fn write_xlsx(
    py: Python<'_>,
    path: PathBuf,
    rows: Bound<'_, PyAny>,
    sheet: Option<String>,
) -> PyResult<()> {
    let grid = py_to_grid(py, &rows)?;
    let name = sheet.unwrap_or_else(|| "Sheet1".to_string());
    write::write_xlsx(&path, &name, &grid).map_err(py_err)
}

#[pyfunction]
#[pyo3(signature = (path, sheet=None))]
fn iter_rows(py: Python<'_>, path: PathBuf, sheet: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyRowIter>> {
    let bytes = xlsx::read_file_bytes(&path).map_err(py_err)?;
    let wb = xlsx::parse_workbook(bytes).map_err(py_err)?;
    let index = match sheet {
        None => 0usize,
        Some(s) => {
            if let Ok(i) = s.extract::<isize>() {
                let n = wb.sheets.len() as isize;
                let idx = if i < 0 { n + i } else { i };
                if idx < 0 || idx >= n {
                    return Err(PyValueError::new_err("sheet index out of range"));
                }
                idx as usize
            } else if let Ok(name) = s.extract::<String>() {
                wb.sheets
                    .iter()
                    .position(|s| s.name == name)
                    .ok_or_else(|| PyValueError::new_err(format!("sheet not found: {name}")))?
            } else {
                return Err(PyValueError::new_err("sheet must be int or str or None"));
            }
        }
    };
    let bytes_ws =
        xlsx::read_worksheet_inflated(&wb, &wb.sheets[index].path).map_err(py_err)?;
    let parser = row_stream::WorksheetRowParser::new(bytes_ws, wb.shared_strings.clone());
    Py::new(py, PyRowIter {
        parser: Some(parser),
    })
}

#[pyfunction]
fn load(py: Python<'_>, path: PathBuf) -> PyResult<Py<PyWorkbook>> {
    let bytes = xlsx::read_file_bytes(&path).map_err(py_err)?;
    let data = xlsx::parse_workbook(bytes).map_err(py_err)?;
    Py::new(
        py,
        PyWorkbook {
            inner: Arc::new(data),
        },
    )
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorkbook>()?;
    m.add_class::<PySheet>()?;
    m.add_class::<PyStreamWriter>()?;
    m.add_class::<PyRowIter>()?;
    m.add_function(wrap_pyfunction!(read_xlsx, m)?)?;
    m.add_function(wrap_pyfunction!(write_xlsx, m)?)?;
    m.add_function(wrap_pyfunction!(iter_rows, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;

    Ok(())
}
