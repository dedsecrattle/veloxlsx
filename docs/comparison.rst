Comparison Guide
================

``veloxlsx`` is intentionally value-focused. It sits between fast read-only
engines and broad Excel authoring libraries: faster and smaller than many
general-purpose options for simple value workflows, but not a replacement for a
full spreadsheet model.

Library Matrix
--------------

.. list-table::
   :header-rows: 1
   :widths: 18 12 12 18 40

   * - Library
     - Read
     - Write
     - Streaming
     - Best fit
   * - ``veloxlsx``
     - Yes
     - Yes
     - Read and write
     - Fast value-oriented XLSX reads and simple exports from Python.
   * - ``python-calamine``
     - Yes
     - No
     - Read-oriented
     - Fast Rust-backed reading across spreadsheet formats.
   * - ``openpyxl``
     - Yes
     - Yes
     - Read/write modes
     - Broad Excel feature coverage in pure Python.
   * - ``pandas``
     - Yes
     - Yes
     - Engine-dependent
     - DataFrame import and export workflows.
   * - ``XlsxWriter``
     - No
     - Yes
     - Constant-memory write mode
     - Rich XLSX generation with formatting and charts.

Decision Guide
--------------

Choose ``veloxlsx`` when:

- you need XLSX values as Python lists;
- row streaming is more important than preserving workbook presentation;
- your export is a plain single-sheet workbook;
- you want a typed, small public API.

Choose ``python-calamine`` when:

- you only need reading;
- you want a mature Rust-backed reader across multiple spreadsheet formats;
- you do not need XLSX writing from the same package.

Choose ``openpyxl`` when:

- preserving or editing Excel workbook structure matters;
- you need styles, formulas, charts, comments, merged cells, or workbook-level manipulation;
- pure Python compatibility is more important than raw read speed.

Choose ``pandas`` when:

- the destination is a DataFrame;
- you need dtype inference, indexing, joins, or analysis operations after import;
- Excel is only one step in a tabular data pipeline.

Choose ``XlsxWriter`` when:

- you only need to generate files;
- formatting, charts, tables, images, or workbook presentation are core requirements;
- read support is unnecessary.

Feature Scope
-------------

.. list-table::
   :header-rows: 1
   :widths: 34 22 44

   * - Feature
     - veloxlsx status
     - Notes
   * - Workbook sheet discovery
     - Supported
     - ``load(path).sheet_names`` exposes sheet order.
   * - Sheet selection
     - Supported
     - Use integer indexes or sheet names.
   * - Shared strings
     - Supported
     - Repeated strings share Rust-side storage where possible.
   * - Inline strings
     - Supported
     - Used by ``StreamWriter`` for bounded memory writes.
   * - Numbers and booleans
     - Supported
     - Returned as Python numeric and boolean values.
   * - Empty cells
     - Supported
     - Returned as ``None`` in the row/grid shape.
   * - Date interpretation
     - Not interpreted yet
     - Date serials may appear as numbers because number formats are not evaluated.
   * - Styles and layout
     - Not applied
     - Fonts, fills, borders, merged cells, dimensions, and similar presentation features are outside the current API.
   * - Formulas
     - Limited value focus
     - The project does not evaluate formulas.
   * - Multi-sheet writing
     - Not supported yet
     - Writers currently create a single sheet.

Benchmark Results
-----------------

The benchmark suite uses a generated numeric workbook so each library reads the
same file. The default large fixture is 4000 x 120 cells, about 480k values.

.. list-table:: Read comparison
   :header-rows: 1
   :widths: 48 26 26

   * - API / library
     - Time (ms)
     - Peak RSS (MiB)
   * - ``veloxlsx.read_xlsx`` (nested lists)
     - 236.3
     - 114.7
   * - ``veloxlsx.iter_rows`` (streaming)
     - 258.8
     - 36.0
   * - ``veloxlsx.load`` + ``read_sheet(0)``
     - 241.0
     - 114.6
   * - openpyxl read-only ``iter_rows``
     - 603.4
     - 38.9
   * - python-calamine ``to_python()``
     - 196.0
     - 68.9
   * - pandas ``read_excel(engine="calamine")``
     - 228.2
     - 141.4
   * - pandas ``read_excel(engine="openpyxl")``
     - 812.6
     - 101.5

.. list-table:: Write comparison
   :header-rows: 1
   :widths: 48 26 26

   * - API / library
     - Time (ms)
     - Peak RSS (MiB)
   * - ``veloxlsx.StreamWriter``
     - 298.3
     - 15.6
   * - ``veloxlsx.write_xlsx``
     - 273.0
     - 70.7
   * - XlsxWriter ``constant_memory``
     - 829.9
     - 24.3

Benchmark Method
----------------

Run the comparison locally:

.. code-block:: bash

   maturin develop --release
   pip install -e ".[dev]"
   python benchmarks/memory_timing.py

To change the grid size:

.. code-block:: bash

   VELOXLSX_BENCH_ROWS=10000 VELOXLSX_BENCH_COLS=200 python benchmarks/memory_timing.py

Each scenario runs in a subprocess and reports wall time plus peak RSS from
``resource.getrusage(RUSAGE_SELF).ru_maxrss``. Optional libraries are skipped if
they are not installed.
