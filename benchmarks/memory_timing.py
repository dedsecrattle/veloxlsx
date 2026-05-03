"""
Measure wall time and peak RSS for large read/write workloads.

Each case runs in a **subprocess** so `ru_maxrss` is meaningful per scenario.

Cross-library read/write rows require optional deps (`pip install -e ".[dev]"`).

  maturin develop --release
  python benchmarks/memory_timing.py

Grid size follows VELOXLSX_BENCH_ROWS / VELOXLSX_BENCH_COLS (default 4000 × 120).
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

_BENCH_DIR = Path(__file__).resolve().parent
if str(_BENCH_DIR) not in sys.path:
    sys.path.insert(0, str(_BENCH_DIR))

from huge_fixture import bench_cols, bench_rows, write_huge_xlsx


def _mod_available(name: str) -> bool:
    return importlib.util.find_spec(name) is not None


def _run_worker(mode: str, args: list[str]) -> tuple[float, float]:
    worker = Path(__file__).resolve().parent / "memory_worker.py"
    cmd = [sys.executable, str(worker), mode, *args]
    proc = subprocess.run(
        cmd,
        check=True,
        capture_output=True,
        text=True,
    )
    data = json.loads(proc.stdout.strip())
    return float(data["ms"]), float(data["peak_rss_mib"])


_READ_DEPS: dict[str, tuple[str, ...]] = {
    "openpyxl_read": ("openpyxl",),
    "calamine_read": ("python_calamine",),
    "pandas_calamine_read": ("pandas",),
    "pandas_openpyxl_read": ("pandas", "openpyxl"),
}


def _print_table(title: str, rows: list[tuple[str, float, float]]) -> None:
    print(f"### {title}")
    print()
    print("| Scenario | Time (ms) | Peak RSS (MiB) |")
    print("|----------|-----------|----------------|")
    for label, ms, rss in rows:
        print(f"| {label} | {ms:,.1f} | {rss:,.1f} |")
    print()


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    rows, cols = bench_rows(), bench_cols()
    tmp = root / ".bench_memory_tmp"
    tmp.mkdir(exist_ok=True)
    input_path = tmp / f"huge_{rows}x{cols}.xlsx"
    if not input_path.is_file():
        print(f"Generating {input_path} ({rows}×{cols})…", file=sys.stderr)
        write_huge_xlsx(input_path, rows, cols)

    path_s = str(input_path)
    read_cases: list[tuple[str, str, list[str]]] = [
        ("veloxlsx `read_xlsx` (nested lists)", "read", [path_s]),
        ("veloxlsx `iter_rows` (streaming; one row at a time)", "iter_rows", [path_s]),
        ("veloxlsx `load` + `read_sheet(0)`", "load_read", [path_s]),
        ("openpyxl read-only `iter_rows`", "openpyxl_read", [path_s]),
        ("python-calamine `to_python()`", "calamine_read", [path_s]),
        (
            "pandas `read_excel` (`engine=\"calamine\"`)",
            "pandas_calamine_read",
            [path_s],
        ),
        (
            "pandas `read_excel` (`engine=\"openpyxl\"`)",
            "pandas_openpyxl_read",
            [path_s],
        ),
    ]

    stream_out = tmp / f"stream_out_{rows}x{cols}.xlsx"
    grid_out = tmp / f"write_xlsx_out_{rows}x{cols}.xlsx"
    xw_out = tmp / f"xlsxwriter_out_{rows}x{cols}.xlsx"

    write_cases: list[tuple[str, str, list[str]]] = [
        (
            "veloxlsx `StreamWriter` (row stream)",
            "stream_write",
            [str(stream_out), str(rows), str(cols)],
        ),
        (
            "veloxlsx `write_xlsx` (grid in memory)",
            "write_xlsx",
            [str(grid_out), str(rows), str(cols)],
        ),
        (
            "XlsxWriter `constant_memory`",
            "xlsxwriter_write",
            [str(xw_out), str(rows), str(cols)],
        ),
    ]

    print(f"Grid: **{rows} × {cols}** (~{rows * cols:,} cells). Python: `{sys.version.split()[0]}`.")
    print()
    print("Peak RSS = `resource.getrusage(RUSAGE_SELF).ru_maxrss` after the workload (subprocess per row).")
    print()

    read_results: list[tuple[str, float, float]] = []
    skipped_read: list[str] = []
    for label, mode, argv in read_cases:
        deps = _READ_DEPS.get(mode, ())
        if any(not _mod_available(m) for m in deps):
            skipped_read.append(label)
            continue
        try:
            ms, rss = _run_worker(mode, argv)
        except subprocess.CalledProcessError as e:
            print(f"[skip] {label}: {e.stderr or e}", file=sys.stderr)
            skipped_read.append(label)
            continue
        read_results.append((label, ms, rss))

    write_results: list[tuple[str, float, float]] = []
    skipped_write: list[str] = []
    for label, mode, argv in write_cases:
        if mode == "xlsxwriter_write" and not _mod_available("xlsxwriter"):
            skipped_write.append(label)
            continue
        try:
            ms, rss = _run_worker(mode, argv)
        except subprocess.CalledProcessError as e:
            print(f"[skip] {label}: {e.stderr or e}", file=sys.stderr)
            skipped_write.append(label)
            continue
        write_results.append((label, ms, rss))

    _print_table("Read same workbook (full sheet)", read_results)
    _print_table("Write same shape (numeric grid)", write_results)

    if skipped_read:
        print("Skipped read (missing dependency or error):", "; ".join(skipped_read))
    if skipped_write:
        print("Skipped write:", "; ".join(skipped_write))


if __name__ == "__main__":
    main()
