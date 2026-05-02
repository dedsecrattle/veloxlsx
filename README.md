# fast-xlsx

Python bindings over a Rust core for reading `.xlsx` (Office Open XML) workbooks. The goal is **read speed** and a small, typed surface area while the format support grows.

## Status (Phase 1)

- Reads workbook structure, **shared strings**, and worksheet cell grids.
- Cell kinds: numbers, booleans, shared strings, inline strings, basic error markers, plain text fallbacks.
- **Dates** are not interpreted from number formats yet; numeric date serials may appear as floats (same caveat as many minimal readers).
- **Styles** (fonts, fills, borders) are not applied to values.

## Phase 2 — write + faster read

- **`fast_xlsx.write_xlsx(path, rows, sheet=...)`** — single-sheet writer with **shared-string deduplication** (memory scales with *unique* strings, not cell count).
- **`fast_xlsx.StreamWriter(path, sheet_name=...)`** — **streaming** writer: call `write_row([...])` repeatedly; uses **inline strings** so memory stays bounded (no giant SST while writing). Supports `with` / `close()`.
- **`fast_xlsx.iter_rows(path, sheet=...)`**, **`Workbook.iter_rows(...)`**, **`Sheet.iter_rows()`** — **streaming read** on the Python side: yields **one row at a time** (each row is a `list` of cell values). For typical workbooks whose cells are wrapped in `<row>` (Excel, XlsxWriter, OpenPyxl), Rust does **not** build a full `rows × cols` grid in memory; peak RSS stays much lower than `read_xlsx`. Sheets that need a **legacy** sparse layout (cells not under `<row>`) fall back internally to buffering like `read_xlsx`.
- **Read path**: one **ZIP archive** is opened during `load()` / `parse_workbook` and **reused** for every sheet read; worksheet XML is parsed from the zip entry stream (no full-sheet `String`). Shared string table entries use **`Arc<str>`** so repeated values clone a pointer, not the text.

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

### Cross-library timing & memory (same fixture)

[`benchmarks/memory_timing.py`](benchmarks/memory_timing.py) runs **one scenario per subprocess** and prints **wall time** (`time.perf_counter`) and **peak RSS** (`resource.getrusage(RUSAGE_SELF).ru_maxrss`, converted to MiB; Linux reports KiB, macOS bytes). Optional libraries are skipped if not installed (`pip install -e ".[dev]"`).

```bash
maturin develop --release
pip install -e ".[dev]"
python benchmarks/memory_timing.py
# optional: FAST_XLSX_BENCH_ROWS=10000 FAST_XLSX_BENCH_COLS=200 python benchmarks/memory_timing.py
```

Sample **read** comparison — same workbook (**4000 × 120** numeric grid, **~480k** cells); **macOS arm64**, **Python 3.13**, **release** `fast-xlsx`, April 2026. Numbers are **indicative** (OS/CPU/RAM/Python build change them).

| API / library | Time (ms) | Peak RSS (MiB) |
|---------------|-----------|----------------|
| fast-xlsx `read_xlsx` (nested lists) | 236.3 | 114.7 |
| fast-xlsx `iter_rows` (streaming; one row at a time) | 258.8 | 36.0 |
| fast-xlsx `load` + `read_sheet(0)` | 241.0 | 114.6 |
| openpyxl read-only `iter_rows` | 603.4 | 38.9 |
| python-calamine `to_python()` | 196.0 | 68.9 |
| pandas `read_excel` (`engine="calamine"`) | 228.2 | 141.4 |
| pandas `read_excel` (`engine="openpyxl"`) | 812.6 | 101.5 |

Sample **write** comparison — generating a **new** file of the same shape (numeric grid):

| API / library | Time (ms) | Peak RSS (MiB) |
|---------------|-----------|----------------|
| fast-xlsx `StreamWriter` (row stream) | 298.3 | 15.6 |
| fast-xlsx `write_xlsx` (grid in Python) | 273.0 | 70.7 |
| XlsxWriter `constant_memory` | 829.9 | 24.3 |

**How to interpret:** higher **peak RSS** usually means the API materialized a large object graph in Python (e.g. `read_xlsx` building a nested list for every cell). **`iter_rows`** avoids holding the whole sheet in Python at once and, for row-based XML, avoids a full Rust grid—here RSS is in the same ballpark as **openpyxl** read-only with **much** lower wall time. **Legacy** sheets may still buffer like `read_xlsx`. Re-run `memory_timing.py` on your machine before choosing.

For pytest micro-benchmarks (not RSS), see `pytest benchmarks/`.

| Library | Read | Write | Excel feature surface |
|---------|------|-------|------------------------|
| **fast-xlsx** | Yes (`read_xlsx`, **`iter_rows`**) | Yes (`write_xlsx`, `StreamWriter`) | Values / basic cell types only (see Status above). |
| **python-calamine** | Yes | No | Read-focused; Rust [calamine](https://github.com/tafia/calamine). |
| **openpyxl** | Yes | Yes | Broad OOXML (styles, charts, …). |
| **pandas** | Yes (`read_excel`) | Yes (`to_excel`, engine-dependent) | DataFrame-centric; uses engines above. |
| **XlsxWriter** | No | Yes | Write-only; rich writing features. |

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
for row in wb.iter_rows("Sheet1"):
    pass  # each row: list of None / bool / int / float / str

fast_xlsx.write_xlsx("out.xlsx", [["a", 1], ["b", 2]], sheet="Data")

with fast_xlsx.StreamWriter("big.xlsx", sheet_name="Sheet1") as w:
    for i in range(1_000_000):
        w.write_row([i, f"row {i}"])

for row in fast_xlsx.iter_rows("book.xlsx", "Data"):
    pass
```

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
