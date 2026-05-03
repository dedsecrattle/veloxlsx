from __future__ import annotations

from pathlib import Path

import pytest

import veloxlsx


def test_write_xlsx_roundtrip(tmp_path: Path) -> None:
    path = tmp_path / "out.xlsx"
    rows = [
        ["hello", 42.5, True, None],
        ["shared", "shared", None, 1],
    ]
    veloxlsx.write_xlsx(path, rows, sheet="Data")
    grid = veloxlsx.read_xlsx(path, "Data")
    assert grid[0][0] == "hello"
    assert grid[0][1] == 42.5
    assert grid[0][2] is True
    assert grid[0][3] is None
    assert grid[1][0] == "shared"
    assert grid[1][1] == "shared"
    assert grid[1][2] is None
    assert grid[1][3] == 1


def test_stream_writer_inline_roundtrip(tmp_path: Path) -> None:
    path = tmp_path / "stream.xlsx"
    with veloxlsx.StreamWriter(path, sheet_name="S") as w:
        w.write_row([1, None, "x"])
        w.write_row([None, 2.5, "y"])
    grid = veloxlsx.read_xlsx(path, "S")
    assert grid[0] == [1, None, "x"]
    assert grid[1] == [None, 2.5, "y"]


def test_load_after_write_sees_single_zip_parse_per_workbook(tmp_path: Path) -> None:
    path = tmp_path / "wb.xlsx"
    veloxlsx.write_xlsx(path, [[1], [2]], sheet="One")
    wb = veloxlsx.load(path)
    assert wb.sheet_names == ["One"]
    assert wb.read_sheet(0) == [[1.0], [2.0]]
