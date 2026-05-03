veloxlsx Documentation
======================

Python bindings over a Rust core for reading `.xlsx` (Office Open XML) workbooks.
The goal is **read speed** and a small, typed surface area while the format support grows.

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   installation
   usage
   architecture
   api

Installation
============

From PyPI
---------

.. code-block:: bash

   pip install veloxlsx

From Source
-----------

Requires Rust, Python 3.10+, and maturin.

.. code-block:: bash

   python -m venv .venv
   source .venv/bin/activate
   pip install maturin
   maturin develop --extras dev

Features
--------

**Phase 1 (Current):**

- Reads workbook structure, shared strings, and worksheet cell grids
- Cell kinds: numbers, booleans, shared strings, inline strings, basic error markers, plain text fallbacks
- Dates are not interpreted from number formats yet
- Styles (fonts, fills, borders) are not applied to values

**Phase 2 (Implemented):**

- ``veloxlsx.write_xlsx(path, rows, sheet=...)`` — single-sheet writer with shared-string deduplication
- ``veloxlsx.StreamWriter(path, sheet_name=...)`` — streaming writer with bounded memory usage
- ``veloxlsx.iter_rows(path, sheet=...)`` — streaming read that yields one row at a time
- Efficient ZIP archive reuse and Arc<str> for shared string table entries

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
