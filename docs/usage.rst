Usage
=====

Reading XLSX Files
------------------

Basic Reading
~~~~~~~~~~~~~

Read an entire sheet as nested lists:

.. code-block:: python

   import veloxlsx

   # Read first sheet
   grid = veloxlsx.read_xlsx("book.xlsx")

   # Read specific sheet by name
   grid = veloxlsx.read_xlsx("book.xlsx", "Sheet2")

   # Read specific sheet by index
   grid = veloxlsx.read_xlsx("book.xlsx", 0)

Workbook API
~~~~~~~~~~~~

Load a workbook for multiple operations:

.. code-block:: python

   import veloxlsx

   wb = veloxlsx.load("book.xlsx")
   
   # Get sheet names
   print(wb.sheet_names)
   
   # Read a sheet by index
   grid = wb.read_sheet(0)
   
   # Access a sheet by name
   sheet = wb["Sheet1"]
   rows = sheet.to_list()

Streaming Read
~~~~~~~~~~~~~~

Stream rows one at a time for memory efficiency:

.. code-block:: python

   import veloxlsx

   # Stream from workbook
   wb = veloxlsx.load("book.xlsx")
   for row in wb.iter_rows("Sheet1"):
       # each row: list of None / bool / int / float / str
       print(row)

   # Stream directly from file
   for row in veloxlsx.iter_rows("book.xlsx", "Data"):
       print(row)

Writing XLSX Files
------------------

Basic Writing
~~~~~~~~~~~~~

Write an entire grid at once:

.. code-block:: python

   import veloxlsx

   veloxlsx.write_xlsx(
       "out.xlsx",
       [["a", 1], ["b", 2]],
       sheet="Data"
   )

Streaming Write
~~~~~~~~~~~~~~~

Stream rows for large files with bounded memory usage:

.. code-block:: python

   import veloxlsx

   with veloxlsx.StreamWriter("big.xlsx", sheet_name="Sheet1") as w:
       for i in range(1_000_000):
           w.write_row([i, f"row {i}"])

Type Annotations
----------------

The package includes full type hints (PEP 561). Use the public API for proper type checking:

.. code-block:: python

   from veloxlsx import CellValue, Grid, Row, read_xlsx

   def process_rows(rows: Grid) -> list[Row]:
       return [list(r) for r in rows]

Cell Values
-----------

Cell values can be one of the following types:

- ``None`` - Empty cells
- ``bool`` - Boolean values
- ``int`` - Integer numbers
- ``float`` - Floating point numbers
- ``str`` - String values (shared strings or inline strings)
