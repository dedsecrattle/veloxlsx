mod error;
mod xlsx;

use error::XlsxError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList, PyModule};
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
        xlsx::CellValue::Text(s) => Ok(s.into_pyobject(py)?.to_owned().into_any().unbind()),
    }
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
    let wb = xlsx::parse_workbook(&bytes).map_err(py_err)?;
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
fn load(py: Python<'_>, path: PathBuf) -> PyResult<Py<PyWorkbook>> {
    let bytes = xlsx::read_file_bytes(&path).map_err(py_err)?;
    let data = xlsx::parse_workbook(&bytes).map_err(py_err)?;
    Py::new(
        py,
        PyWorkbook {
            inner: Arc::new(data),
        },
    )
}

#[pymodule]
fn fast_xlsx(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWorkbook>()?;
    m.add_class::<PySheet>()?;
    m.add_function(wrap_pyfunction!(read_xlsx, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;

    Ok(())
}
