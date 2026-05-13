API Reference
=============

Classes
-------

Workbook
^^^^^^^^

.. autoclass:: veloxlsx.Workbook
   :members:
   :undoc-members:

Sheet
^^^^^

.. autoclass:: veloxlsx.Sheet
   :members:
   :undoc-members:

RowIter
^^^^^^^

.. autoclass:: veloxlsx.RowIter
   :members:
   :undoc-members:

StreamWriter
^^^^^^^^^^^^

.. autoclass:: veloxlsx.StreamWriter
   :members:
   :undoc-members:

Functions
---------

read_xlsx
^^^^^^^^^

.. autofunction:: veloxlsx.read_xlsx

write_xlsx
^^^^^^^^^^

.. autofunction:: veloxlsx.write_xlsx

iter_rows
^^^^^^^^^

.. autofunction:: veloxlsx.iter_rows

load
^^^^

.. autofunction:: veloxlsx.load

Type Aliases
------------

.. py:data:: veloxlsx.CellValue
   :type: None | bool | int | float | str

   A supported cell value returned by readers and accepted by writers.

.. py:data:: veloxlsx.Row
   :type: list[CellValue]

   A worksheet row represented as a Python list.

.. py:data:: veloxlsx.Grid
   :type: list[Row]

   A materialized worksheet represented as nested Python lists.
