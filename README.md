# veloxlsx

[![PyPI Version](https://img.shields.io/pypi/v/veloxlsx)](https://pypi.org/project/veloxlsx/)
[![Python Versions](https://img.shields.io/pypi/pyversions/veloxlsx)](https://pypi.org/project/veloxlsx/)
[![CI](https://github.com/dedsecrattle/fast-xlsx/actions/workflows/ci.yml/badge.svg)](https://github.com/dedsecrattle/fast-xlsx/actions/workflows/ci.yml)
[![Documentation](https://github.com/dedsecrattle/fast-xlsx/actions/workflows/docs.yml/badge.svg)](https://dedsecrattle.github.io/fast-xlsx/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-APACHE)

`veloxlsx` is a Python XLSX reader and writer backed by a Rust core. It is built for data workflows that need a small typed API, fast workbook reads, and low-memory row streaming without pulling in a full spreadsheet object model.

```bash
pip install veloxlsx
```

```python
import veloxlsx

rows = veloxlsx.read_xlsx("book.xlsx", "Data")

with veloxlsx.StreamWriter("out.xlsx", sheet_name="Export") as writer:
    for i, row in enumerate(rows):
        writer.write_row([i, *row])
```

Full documentation: [dedsecrattle.github.io/fast-xlsx](https://dedsecrattle.github.io/fast-xlsx/)

## Why veloxlsx?

- **Fast Rust core** for parsing workbook metadata, shared strings, and sheet XML.
- **Streaming reads** with `iter_rows()` when you want one Python row at a time.
- **Two write paths**: `write_xlsx()` for in-memory grids and `StreamWriter` for bounded-memory exports.
- **Typed Python package** with PEP 561 type information.
- **Focused feature surface** for values and basic cell types rather than styles, charts, formulas, or workbook editing.

## Feature Snapshot

| Capability | Status |
|------------|--------|
| Read `.xlsx` workbooks | Supported |
| Select sheets by name or index | Supported |
| Stream rows from files or loaded workbooks | Supported |
| Write single-sheet workbooks | Supported |
| Stream large XLSX exports | Supported |
| Type hints | Supported |
| Dates from number formats | Not interpreted yet |
| Styles, charts, merged cells, formulas | Not part of the current value-focused API |

Cell values are returned as `None`, `bool`, `int`, `float`, or `str`.

## Quick Start

### Read a workbook

```python
import veloxlsx

grid = veloxlsx.read_xlsx("book.xlsx")          # first sheet
grid = veloxlsx.read_xlsx("book.xlsx", "Data")  # by sheet name
grid = veloxlsx.read_xlsx("book.xlsx", 0)       # by sheet index
```

### Reuse a loaded workbook

```python
import veloxlsx

workbook = veloxlsx.load("book.xlsx")
print(workbook.sheet_names)

rows = workbook.read_sheet("Data")
sheet = workbook["Data"]
same_rows = sheet.to_list()
```

### Stream rows

```python
import veloxlsx

for row in veloxlsx.iter_rows("large.xlsx", "Data"):
    process(row)
```

### Write a workbook

```python
import veloxlsx

veloxlsx.write_xlsx("out.xlsx", [["name", "count"], ["apples", 12]], sheet="Data")
```

### Stream a large export

```python
import veloxlsx

with veloxlsx.StreamWriter("large-export.xlsx", sheet_name="Rows") as writer:
    for i in range(1_000_000):
        writer.write_row([i, f"row {i}"])
```

## Comparison

Choose `veloxlsx` when you need fast value extraction or simple XLSX exports from Python. Choose a broader spreadsheet library when you need rich Excel authoring or workbook manipulation.

| Library | Read | Write | Streaming | Best fit |
|---------|------|-------|-----------|----------|
| **veloxlsx** | Yes | Yes | Read and write | Fast value-oriented reads and simple exports |
| **python-calamine** | Yes | No | Read-oriented | Fast Rust-backed reading across spreadsheet formats |
| **openpyxl** | Yes | Yes | Read/write modes | Broad Excel feature coverage in pure Python |
| **pandas** | Yes | Yes | Engine-dependent | DataFrame import/export workflows |
| **XlsxWriter** | No | Yes | Constant-memory write mode | Rich XLSX generation |

More detail is available in the [comparison guide](https://dedsecrattle.github.io/fast-xlsx/comparison.html).

## Benchmarks

The benchmark suite compares `veloxlsx` with `openpyxl`, `python-calamine`, `pandas`, and `XlsxWriter` using the same generated workbook.

```bash
maturin develop --release
pip install -e ".[dev]"
python benchmarks/memory_timing.py
```

Sample results for a `4000 x 120` numeric grid, about 480k cells, on macOS arm64 with Python 3.13 and a release build:

| Read API | Time (ms) | Peak RSS (MiB) |
|----------|-----------|----------------|
| veloxlsx `read_xlsx` | 236.3 | 114.7 |
| veloxlsx `iter_rows` | 258.8 | 36.0 |
| veloxlsx `load` + `read_sheet(0)` | 241.0 | 114.6 |
| openpyxl read-only `iter_rows` | 603.4 | 38.9 |
| python-calamine `to_python()` | 196.0 | 68.9 |
| pandas `read_excel(engine="calamine")` | 228.2 | 141.4 |
| pandas `read_excel(engine="openpyxl")` | 812.6 | 101.5 |

| Write API | Time (ms) | Peak RSS (MiB) |
|-----------|-----------|----------------|
| veloxlsx `StreamWriter` | 298.3 | 15.6 |
| veloxlsx `write_xlsx` | 273.0 | 70.7 |
| XlsxWriter `constant_memory` | 829.9 | 24.3 |

Benchmarks are workload- and machine-dependent. Re-run them on your target environment before making library choices based on latency or memory.

## Installation

### From PyPI

```bash
pip install veloxlsx
```

### From Source

Requires Rust, Python 3.10+, and [maturin](https://www.maturin.rs/).

```bash
python -m venv .venv
source .venv/bin/activate
pip install maturin
maturin develop --extras dev
```

## Development

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
maturin develop --release
pytest
```

Build the docs locally:

```bash
pip install -e ".[docs]"
sphinx-build -b html docs docs/_build/html
```

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
