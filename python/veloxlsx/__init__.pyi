"""
Type information for :mod:`veloxlsx`.

Runtime implementation lives in :mod:`veloxlsx._native` (Rust extension).
"""

from __future__ import annotations

from collections.abc import Sequence
from os import PathLike
from typing import overload, TypeAlias

StrPath: TypeAlias = str | PathLike[str]

CellValue: TypeAlias = None | bool | int | float | str
Row: TypeAlias = list[CellValue]
Grid: TypeAlias = list[Row]

class Workbook:
    @property
    def sheet_names(self) -> list[str]: ...
    def __len__(self) -> int: ...
    @overload
    def __getitem__(self, key: int) -> Sheet: ...
    @overload
    def __getitem__(self, key: str) -> Sheet: ...
    @overload
    def read_sheet(self, sheet: int) -> Grid: ...
    @overload
    def read_sheet(self, sheet: str) -> Grid: ...
    @overload
    def iter_rows(self, sheet: int) -> RowIter: ...
    @overload
    def iter_rows(self, sheet: str) -> RowIter: ...

class Sheet:
    @property
    def name(self) -> str: ...
    def to_list(self) -> Grid: ...
    def iter_rows(self) -> RowIter: ...

class RowIter:
    def __iter__(self) -> RowIter: ...
    def __next__(self) -> Row: ...

class StreamWriter:
    def __init__(
        self,
        path: StrPath,
        sheet_name: str | None = None,
    ) -> None: ...
    def write_row(self, row: Sequence[CellValue]) -> None: ...
    def close(self) -> None: ...
    def __enter__(self) -> StreamWriter: ...
    def __exit__(self, *args: object) -> None: ...

@overload
def read_xlsx(path: StrPath) -> Grid: ...
@overload
def read_xlsx(path: StrPath, sheet: int) -> Grid: ...
@overload
def read_xlsx(path: StrPath, sheet: str) -> Grid: ...

def write_xlsx(
    path: StrPath,
    rows: Sequence[Sequence[CellValue]],
    sheet: str | None = None,
) -> None: ...

@overload
def iter_rows(path: StrPath) -> RowIter: ...
@overload
def iter_rows(path: StrPath, sheet: int) -> RowIter: ...
@overload
def iter_rows(path: StrPath, sheet: str) -> RowIter: ...

def load(path: StrPath) -> Workbook: ...

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
