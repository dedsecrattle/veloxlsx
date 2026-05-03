Architecture and Performance
=============================

Reading Modes Comparison
-------------------------

veloxlsx provides multiple reading modes optimized for different use cases:

.. graphviz::

    digraph read_modes {
        rankdir=LR;
        node [shape=box, style=filled, fontname="Arial"];
        
        subgraph cluster_read_xlsx {
            label = "read_xlsx()";
            style=filled;
            color=lightblue;
            file [label="XLSX File"];
            zip [label="Open ZIP"];
            parse [label="Parse XML"];
            grid [label="Build Full Grid\nin Rust"];
            python [label="Convert to\nPython Lists"];
            file -> zip -> parse -> grid -> python;
        }
        
        subgraph cluster_iter_rows {
            label = "iter_rows()";
            style=filled;
            color=lightgreen;
            file2 [label="XLSX File"];
            zip2 [label="Open ZIP"];
            parse2 [label="Stream Parse\nRow by Row"];
            yield [label="Yield Row\nto Python"];
            file2 -> zip2 -> parse2 -> yield;
        }
        
        subgraph cluster_load {
            label = "load() + read_sheet()";
            style=filled, color=lightyellow;
            file3 [label="XLSX File"];
            zip3 [label="Open ZIP\n(Reuse for all sheets)"];
            wb [label="Workbook Object"];
            sheet [label="Read Sheet\non Demand"];
            file3 -> zip3 -> wb -> sheet;
        }
    }

**Memory Usage Comparison:**

.. graphviz::

    digraph memory {
        rankdir=TB;
        node [shape=box, style=filled, fontname="Arial"];
        
        subgraph cluster_memory {
            label = "Peak Memory Usage (Relative)";
            style=filled;
            
            read_xlsx [label="read_xlsx()\nHigh", fillcolor="#ff6b6b"];
            iter_rows [label="iter_rows()\nLow", fillcolor="#51cf66"];
            load_read [label="load() + read_sheet()\nHigh", fillcolor="#ff6b6b"];
            openpyxl [label="openpyxl iter_rows()\nLow", fillcolor="#ffd43b"];
            
            read_xlsx -> iter_rows;
            iter_rows -> openpyxl;
            openpyxl -> load_read;
        }
    }

**When to use each mode:**

- **`read_xlsx()`**: Best for small to medium files when you need the entire dataset in memory at once. Simple API, but higher memory usage.
- **`iter_rows()`**: Best for large files or streaming processing. Yields one row at a time, keeping memory usage low regardless of file size.
- **`load()` + `read_sheet()`**: Best when you need to work with multiple sheets from the same workbook. The ZIP archive is opened once and reused.

Writing Modes Comparison
------------------------

veloxlsx provides two writing modes with different memory characteristics:

.. graphviz::

    digraph write_modes {
        rankdir=LR;
        node [shape=box, style=filled, fontname="Arial"];
        
        subgraph cluster_write_xlsx {
            label = "write_xlsx()";
            style=filled;
            color=lightblue;
            data [label="Python Grid\n(List of Lists)"];
            dedup [label="Deduplicate\nShared Strings"];
            sst [label="Build Shared\nString Table"];
            write [label="Write XLSX"];
            data -> dedup -> sst -> write;
        }
        
        subgraph cluster_stream_writer {
            label = "StreamWriter";
            style=filled;
            color=lightgreen;
            data2 [label="Stream Rows\nOne at a Time"];
            inline [label="Use Inline\nStrings"];
            write2 [label="Write XLSX\nIncrementally"];
            data2 -> inline -> write2;
        }
    }

**Memory Usage Comparison:**

.. graphviz::

    digraph write_memory {
        rankdir=TB;
        node [shape=box, style=filled, fontname="Arial"];
        
        subgraph cluster_write_mem {
            label = "Peak Memory Usage (Relative)";
            style=filled;
            
            write_xlsx [label="write_xlsx()\nMedium", fillcolor="#ffd43b"];
            stream [label="StreamWriter()\nVery Low", fillcolor="#51cf66"];
            xlsxwriter [label="XlsxWriter\nconstant_memory\nLow", fillcolor="#51cf66"];
            
            write_xlsx -> xlsxwriter;
            xlsxwriter -> stream;
        }
    }

**When to use each mode:**

- **`write_xlsx()`**: Best when you have the complete dataset in memory. Uses shared string deduplication, which reduces file size but requires building a string table in memory.
- **`StreamWriter`**: Best for writing very large files or when data comes incrementally. Uses inline strings to keep memory bounded during writing. File size may be larger due to no string deduplication.

Performance Characteristics
----------------------------

Based on benchmark results (4000 × 120 numeric grid, ~480k cells):

.. image:: _static/benchmark_comparison.png
   :alt: Benchmark comparison charts
   :align: center
   :width: 100%

.. image:: _static/benchmark_scatter.png
   :alt: Time vs memory tradeoff scatter plot
   :align: center
   :width: 80%

**Read Performance:**

| Method | Time (ms) | Memory (MiB) | Best For |
|--------|-----------|--------------|----------|
| `read_xlsx()` | 236 | 115 | Small files, simple API |
| `iter_rows()` | 259 | 36 | Large files, streaming |
| `load()` + `read_sheet()` | 241 | 115 | Multi-sheet workbooks |
| python-calamine | 196 | 69 | Read-only, Rust-native |
| openpyxl read-only | 603 | 39 | Feature-rich reading |

**Write Performance:**

| Method | Time (ms) | Memory (MiB) | Best For |
|--------|-----------|--------------|----------|
| `write_xlsx()` | 273 | 71 | Medium files, smaller output |
| `StreamWriter` | 298 | 16 | Large files, streaming |
| XlsxWriter constant_memory | 830 | 24 | Feature-rich writing |

**Key Insights:**

1. **Streaming modes** (`iter_rows`, `StreamWriter`) trade slight speed for significant memory savings
2. **veloxlsx** is competitive with other Rust-based libraries (python-calamine) while offering write capabilities
3. **Shared string deduplication** in `write_xlsx()` reduces file size but increases memory usage
4. **ZIP archive reuse** in `load()` makes multi-sheet operations efficient

Benchmarking Very Large Files
------------------------------

To benchmark with very large files (e.g., 100k rows × 500 columns = 50M cells):

.. code-block:: bash

   # Set environment variables for large file size
   export VELOXLSX_BENCH_ROWS=100000
   export VELOXLSX_BENCH_COLS=500
   
   # Generate the large test file
   python benchmarks/huge_fixture.py
   
   # Run memory timing benchmarks
   python benchmarks/memory_timing.py
   
   # Update visualization script with new results
   # Edit benchmarks/visualize_results.py with your data
   
   # Regenerate charts
   python benchmarks/visualize_results.py

See ``benchmarks/large_file_benchmark_template.py`` for a template script for benchmarking very large files.
