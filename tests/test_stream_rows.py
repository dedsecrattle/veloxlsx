from __future__ import annotations

from pathlib import Path

import pytest

import fast_xlsx


def test_iter_rows_matches_read_xlsx(sample_xlsx_path: Path) -> None:
    dense = fast_xlsx.read_xlsx(sample_xlsx_path, "Data")
    rows = list(fast_xlsx.iter_rows(sample_xlsx_path, "Data"))
    assert rows == dense


def test_workbook_iter_rows(sample_xlsx_path: Path) -> None:
    wb = fast_xlsx.load(sample_xlsx_path)
    dense = wb.read_sheet("Data")
    rows = list(wb.iter_rows("Data"))
    assert rows == dense


def test_sheet_iter_rows(sample_xlsx_path: Path) -> None:
    wb = fast_xlsx.load(sample_xlsx_path)
    rows = list(wb["Data"].iter_rows())
    assert rows == wb.read_sheet("Data")
