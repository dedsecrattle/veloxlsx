Installation
============

Requirements
------------

``veloxlsx`` supports Python 3.10 and newer. Published wheels are built from a
Rust extension module and expose a normal Python package named ``veloxlsx``.

Install from PyPI
-----------------

.. code-block:: bash

   pip install veloxlsx

Verify the install:

.. code-block:: python

   import veloxlsx

   print(veloxlsx.__all__)

Build from Source
-----------------

Source builds require Rust, Python 3.10 or newer, and ``maturin``.

.. code-block:: bash

   python -m venv .venv
   source .venv/bin/activate
   python -m pip install --upgrade pip
   pip install maturin
   maturin develop --extras dev

Development Environment
-----------------------

Install the optional development dependencies when running tests, benchmarks,
or comparison scripts:

.. code-block:: bash

   pip install -e ".[dev]"
   maturin develop --release
   pytest

The development extra includes ``pytest``, ``pytest-benchmark``, ``openpyxl``,
``pandas``, ``python-calamine``, ``xlsxwriter``, and ``matplotlib``.

Documentation Environment
-------------------------

Install the docs extra:

.. code-block:: bash

   pip install -e ".[docs]"
   sphinx-build -b html docs docs/_build/html

The Sphinx docs use Graphviz for architecture diagrams. If diagram rendering
fails locally, install the system package:

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - Platform
     - Command
   * - macOS
     - ``brew install graphviz``
   * - Ubuntu/Debian
     - ``sudo apt-get install graphviz``
   * - Windows
     - Install from https://graphviz.org/download/ or run ``choco install graphviz``.
