Usage
=====

``veloxlsx`` exposes a small API around two read styles and two write styles.
Use the full-grid APIs when the sheet is comfortably sized for memory. Use the
streaming APIs when rows should be processed incrementally.

Reading Workbooks
-----------------

Read an entire sheet as nested Python lists:

.. code-block:: python

   import veloxlsx

   first_sheet = veloxlsx.read_xlsx("book.xlsx")
   by_name = veloxlsx.read_xlsx("book.xlsx", "Data")
   by_index = veloxlsx.read_xlsx("book.xlsx", 0)

The returned value is a ``list[list[CellValue]]`` where ``CellValue`` is
``None | bool | int | float | str``.

Loaded Workbook API
-------------------

Use ``load`` when you need sheet names or multiple operations on the same file.
The workbook keeps parsed workbook metadata and reuses the underlying ZIP
archive for sheet reads.

.. code-block:: python

   import veloxlsx

   workbook = veloxlsx.load("book.xlsx")
   print(workbook.sheet_names)

   rows = workbook.read_sheet("Data")

   sheet = workbook["Data"]
   same_rows = sheet.to_list()

Streaming Reads
---------------

``iter_rows`` yields one row at a time. For typical workbooks whose cells are
wrapped in row elements, this avoids building a complete Rust-side grid and a
complete Python nested list.

.. code-block:: python

   import veloxlsx

   for row in veloxlsx.iter_rows("large.xlsx", "Data"):
       handle(row)

The same iterator is available from loaded workbooks and sheets:

.. code-block:: python

   workbook = veloxlsx.load("large.xlsx")

   for row in workbook.iter_rows("Data"):
       handle(row)

   sheet = workbook["Data"]
   for row in sheet.iter_rows():
       handle(row)

Some legacy sparse worksheet layouts may still require internal buffering.

Writing Workbooks
-----------------

Use ``write_xlsx`` when you already have all rows in memory:

.. code-block:: python

   import veloxlsx

   rows = [
       ["name", "count"],
       ["apples", 12],
       ["oranges", 8],
   ]

   veloxlsx.write_xlsx("inventory.xlsx", rows, sheet="Inventory")

``write_xlsx`` builds a shared string table, so repeated strings are deduplicated
inside the XLSX file.

Streaming Writes
----------------

Use ``StreamWriter`` when rows are produced incrementally or the export is too
large to keep as a Python list.

.. code-block:: python

   import veloxlsx

   with veloxlsx.StreamWriter("events.xlsx", sheet_name="Events") as writer:
       writer.write_row(["id", "message"])
       for i in range(1_000_000):
           writer.write_row([i, f"event {i}"])

``StreamWriter`` writes inline strings to keep memory bounded. The resulting
file may be larger than a shared-string workbook when the same string repeats
many times.

Cell Values
-----------

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - Python type
     - Meaning
   * - ``None``
     - Empty cell.
   * - ``bool``
     - Boolean cell value.
   * - ``int``
     - Integer value where the parsed numeric value is integral.
   * - ``float``
     - Floating point value or numeric date serial.
   * - ``str``
     - Shared string, inline string, error marker, or plain text fallback.

Type Annotations
----------------

The package ships PEP 561 type information:

.. code-block:: python

   from veloxlsx import CellValue, Grid, Row

   def drop_empty_rows(rows: Grid) -> list[Row]:
       return [row for row in rows if any(value is not None for value in row)]
