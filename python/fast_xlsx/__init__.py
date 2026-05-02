"""XLSX read/write backed by a Rust core (``fast_xlsx._native``)."""

from __future__ import annotations

from typing import TypeAlias

from fast_xlsx._native import (
    RowIter,
    Sheet,
    StreamWriter,
    Workbook,
    iter_rows,
    load,
    read_xlsx,
    write_xlsx,
)

CellValue: TypeAlias = None | bool | int | float | str
Row: TypeAlias = list[CellValue]
Grid: TypeAlias = list[Row]

__all__ = [
    "CellValue",
    "Grid",
    "Row",
    "RowIter",
    "Sheet",
    "StreamWriter",
    "Workbook",
    "iter_rows",
    "load",
    "read_xlsx",
    "write_xlsx",
]
