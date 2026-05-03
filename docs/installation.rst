Installation
============

From PyPI
---------

The easiest way to install veloxlsx is via pip:

.. code-block:: bash

   pip install veloxlsx

From Source
-----------

If you want to build from source, you'll need Rust, Python 3.10+, and maturin:

.. code-block:: bash

   python -m venv .venv
   source .venv/bin/activate
   pip install maturin
   maturin develop --extras dev

Development Dependencies
-------------------------

The package includes optional development dependencies for testing and benchmarking:

.. code-block:: bash

   pip install -e ".[dev]"

This includes:

- pytest and pytest-benchmark for testing
- openpyxl, pandas, python-calamine, and xlsxwriter for benchmarking comparisons
