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

Building Documentation
----------------------

To build the documentation locally, you need:

1. **Graphviz** (system package for rendering diagrams):

   **macOS:**

   .. code-block:: bash

      brew install graphviz

   **Ubuntu/Debian:**

   .. code-block:: bash

      sudo apt-get install graphviz

   **Windows:**

   Download from https://graphviz.org/download/ or use Chocolatey:

   .. code-block:: bash

      choco install graphviz

2. **Python documentation dependencies:**

   .. code-block:: bash

      pip install -r docs/requirements.txt

3. **Build the docs:**

   .. code-block:: bash

      cd docs
      make html

   The built documentation will be in ``docs/_build/html/``.
