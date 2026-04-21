# fast-xlsx

Python bindings over a Rust core for reading `.xlsx` (Office Open XML) workbooks. The goal is **read speed** and a small, typed surface area while the format support grows.

## Status (Phase 1)

- Reads workbook structure, **shared strings**, and worksheet cell grids.
- Cell kinds: numbers, booleans, shared strings, inline strings, basic error markers, plain text fallbacks.
- **Dates** are not interpreted from number formats yet; numeric date serials may appear as floats (same caveat as many minimal readers).
- **Styles** (fonts, fills, borders) are not applied to values.

## Benchmarks (large files vs other libraries)

The default unit run (`pytest`) only hits [`tests/`](tests/). For a **large grid** comparison against **openpyxl**, **python-calamine** (Rust calamine), and **pandas** (`read_excel` with `calamine` vs `openpyxl` engines), use the separate suite under [`benchmarks/`](benchmarks/):

```bash
maturin develop --release
pip install -e ".[dev]"   # includes xlsxwriter, pandas, python-calamine, pytest-benchmark
pytest benchmarks/
```

Grid size (defaults **4000 × 120** cells ≈ **480k** values):

```bash
FAST_XLSX_BENCH_ROWS=10000 FAST_XLSX_BENCH_COLS=200 pytest benchmarks/
```

The workbook is generated with **xlsxwriter** (fast streaming write) so you are mostly measuring **read** performance, not fixture build time after the first module-scoped write.

## Install (from source)

Requires Rust, Python 3.10+, and [maturin](https://www.maturin.rs/).

```bash
python -m venv .venv
source .venv/bin/activate
pip install maturin
maturin develop --extras dev
```

## Usage

```python
import fast_xlsx

grid = fast_xlsx.read_xlsx("book.xlsx")  # first sheet
grid = fast_xlsx.read_xlsx("book.xlsx", "Sheet2")
grid = fast_xlsx.read_xlsx("book.xlsx", 0)

wb = fast_xlsx.load("book.xlsx")
assert wb.sheet_names[0] == "Sheet1"
same = wb.read_sheet(0)
sheet = wb["Sheet1"]
rows = sheet.to_list()
```

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
