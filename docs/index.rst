veloxlsx
========

Fast, typed XLSX reading and writing for Python, backed by a Rust core.

``veloxlsx`` is designed for data workflows that need to read workbook values
quickly, stream large sheets row by row, or write simple XLSX exports without a
large spreadsheet object model.

.. code-block:: bash

   pip install veloxlsx

.. code-block:: python

   import veloxlsx

   rows = veloxlsx.read_xlsx("book.xlsx", "Data")

   with veloxlsx.StreamWriter("export.xlsx", sheet_name="Rows") as writer:
       for row in rows:
           writer.write_row(row)

Project Focus
-------------

.. list-table::
   :header-rows: 1
   :widths: 24 46 30

   * - Area
     - What veloxlsx does
     - Notes
   * - Reading
     - Reads workbook metadata, shared strings, and worksheet cell values.
     - Use ``read_xlsx`` for full grids or ``iter_rows`` for streaming.
   * - Writing
     - Writes single-sheet workbooks from grids or row streams.
     - Use ``StreamWriter`` for large exports.
   * - Types
     - Returns ``None``, ``bool``, ``int``, ``float``, and ``str`` values.
     - Type hints are shipped with the package.
   * - Scope
     - Focuses on values and basic cell types.
     - Dates, styles, charts, merged cells, and formulas are not interpreted as rich Excel objects yet.

When to Use It
--------------

Use ``veloxlsx`` when you want:

- a small Python API for reading XLSX values;
- low-memory row iteration over large sheets;
- simple XLSX export without styling or workbook editing;
- type hints for application code.

Use a richer spreadsheet package when you need to preserve formatting, charts,
formula semantics, worksheet layout, or advanced Excel authoring features.

Performance Snapshot
--------------------

Indicative results for a 4000 x 120 numeric workbook, about 480k cells, on
macOS arm64 with Python 3.13 and a release build:

.. list-table::
   :header-rows: 1
   :widths: 46 27 27

   * - Read API
     - Time (ms)
     - Peak RSS (MiB)
   * - ``veloxlsx.read_xlsx``
     - 236.3
     - 114.7
   * - ``veloxlsx.iter_rows``
     - 258.8
     - 36.0
   * - openpyxl read-only ``iter_rows``
     - 603.4
     - 38.9
   * - python-calamine ``to_python()``
     - 196.0
     - 68.9

Re-run the benchmark suite on your own target machine before choosing a library
based on latency or memory.

Documentation
-------------

.. toctree::
   :maxdepth: 2
   :caption: Guides

   installation
   usage
   comparison
   architecture
   api

Indices and Tables
------------------

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
