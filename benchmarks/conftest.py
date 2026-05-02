"""Fixtures for optional large-file read benchmarks (not collected by default `pytest`)."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

_BENCH_DIR = Path(__file__).resolve().parent
if str(_BENCH_DIR) not in sys.path:
    sys.path.insert(0, str(_BENCH_DIR))

from huge_fixture import bench_cols, bench_rows, write_huge_xlsx


@pytest.fixture(scope="module")
def huge_xlsx_path(tmp_path_factory: pytest.TempPathFactory) -> Path:
    rows, cols = bench_rows(), bench_cols()
    base = tmp_path_factory.mktemp("huge_xlsx")
    path = base / f"huge_{rows}x{cols}.xlsx"
    if not path.is_file():
        write_huge_xlsx(path, rows, cols)
    return path


@pytest.fixture(scope="module")
def huge_cell_count() -> int:
    return bench_rows() * bench_cols()
