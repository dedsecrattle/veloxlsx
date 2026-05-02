"""Shared helpers for large XLSX benchmark fixtures (rows × cols numeric grid)."""

from __future__ import annotations

import os
from pathlib import Path


def bench_rows() -> int:
    return max(50, int(os.environ.get("FAST_XLSX_BENCH_ROWS", "4000")))


def bench_cols() -> int:
    return max(10, int(os.environ.get("FAST_XLSX_BENCH_COLS", "120")))


def write_huge_xlsx(path: Path, rows: int, cols: int) -> None:
    import xlsxwriter

    path.parent.mkdir(parents=True, exist_ok=True)
    wb = xlsxwriter.Workbook(path, {"constant_memory": True})
    ws = wb.add_worksheet("Sheet1")
    for r in range(rows):
        ws.write_row(r, 0, [r * 1_000_000 + c for c in range(cols)])
    wb.close()
