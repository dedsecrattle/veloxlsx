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
| fast-xlsx `read_xlsx` (nested lists) | 246.0 | 114.6 |
| fast-xlsx `load` + `read_sheet(0)` | 247.1 | 114.4 |
| openpyxl read-only `iter_rows` | 587.1 | 39.0 |
| python-calamine `to_python()` | 198.6 | 68.9 |
| pandas `read_excel` (`engine="calamine"`) | 230.0 | 141.5 |
| pandas `read_excel` (`engine="openpyxl"`) | 799.2 | 101.4 |

Sample **write** comparison — generating a **new** file of the same shape (numeric grid):

| API / library | Time (ms) | Peak RSS (MiB) |
|---------------|-----------|----------------|
| fast-xlsx `StreamWriter` (row stream) | 296.7 | 15.6 |
| fast-xlsx `write_xlsx` (grid in Python) | 270.4 | 70.6 |
| XlsxWriter `constant_memory` | 818.3 | 24.3 |

**How to interpret:** higher **peak RSS** usually means the API materialized a large object graph in Python (e.g. fast-xlsx and pandas building full structures). **openpyxl** in read-only mode streams rows, so RSS stays lower but wall time is higher here. **fast-xlsx** `StreamWriter` keeps RSS low; wall time vs **XlsxWriter** on this run favors the Rust path—re-run on your machine before choosing.

For pytest micro-benchmarks (not RSS), see `pytest benchmarks/`.

| Library | Read | Write | Excel feature surface |
|---------|------|-------|------------------------|
| **fast-xlsx** | Yes | Yes (`write_xlsx`, `StreamWriter`) | Values / basic cell types only (see Status above). |
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

fast_xlsx.write_xlsx("out.xlsx", [["a", 1], ["b", 2]], sheet="Data")

with fast_xlsx.StreamWriter("big.xlsx", sheet_name="Sheet1") as w:
    for i in range(1_000_000):
        w.write_row([i, f"row {i}"])
```

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
