"""Fixtures for optional large-file read benchmarks (not collected by default `pytest`)."""

from __future__ import annotations

import os
from pathlib import Path

import pytest


def _bench_rows() -> int:
    return max(50, int(os.environ.get("FAST_XLSX_BENCH_ROWS", "4000")))


def _bench_cols() -> int:
    return max(10, int(os.environ.get("FAST_XLSX_BENCH_COLS", "120")))


def _write_huge_xlsx(path: Path, rows: int, cols: int) -> None:
    import xlsxwriter

    path.parent.mkdir(parents=True, exist_ok=True)
    wb = xlsxwriter.Workbook(path, {"constant_memory": True})
    ws = wb.add_worksheet("Sheet1")
    for r in range(rows):
        ws.write_row(r, 0, [r * 1_000_000 + c for c in range(cols)])
    wb.close()


@pytest.fixture(scope="module")
def huge_xlsx_path(tmp_path_factory: pytest.TempPathFactory) -> Path:
    rows, cols = _bench_rows(), _bench_cols()
    base = tmp_path_factory.mktemp("huge_xlsx")
    path = base / f"huge_{rows}x{cols}.xlsx"
    if not path.is_file():
        _write_huge_xlsx(path, rows, cols)
    return path


@pytest.fixture(scope="module")
def huge_cell_count() -> int:
    return _bench_rows() * _bench_cols()
