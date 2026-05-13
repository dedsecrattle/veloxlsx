# Building Documentation

This directory contains the Sphinx documentation for veloxlsx.

## Prerequisites

### System Requirements

1. **Graphviz** (for rendering diagrams in the documentation):
   
   **macOS:**
   ```bash
   brew install graphviz
   ```
   
   **Ubuntu/Debian:**
   ```bash
   sudo apt-get install graphviz
   ```
   
   **Windows:**
   Download from https://graphviz.org/download/ or use:
   ```bash
   choco install graphviz
   ```

### Python Requirements

Install the documentation dependencies:

```bash
pip install -r docs/requirements.txt
```

## Building the Documentation

From the repository root:

```bash
pip install -e ".[docs]"
sphinx-build -b html docs docs/_build/html
```

The built documentation will be in `docs/_build/html/`. Open `docs/_build/html/index.html` in your browser.

The public GitHub Pages site is built from:

- `index.rst` - project overview and navigation
- `usage.rst` - read/write examples
- `comparison.rst` - library comparison and benchmark tables
- `architecture.rst` - read/write architecture and performance notes
- `api.rst` - generated API reference

## Updating Benchmark Images

The benchmark comparison images in `_static/` are generated from the visualization script:

```bash
# Run benchmarks first
python benchmarks/memory_timing.py

# Generate images (requires matplotlib)
pip install matplotlib
python benchmarks/visualize_results.py

# Copy to docs
cp benchmarks/benchmark_comparison.png benchmarks/benchmark_scatter.png docs/_static/
```

## Troubleshooting

### Graphviz diagrams not rendering

If you see errors like "graphviz not found" or diagrams appear as raw text:

1. Ensure Graphviz is installed system-wide (see Prerequisites above)
2. Verify it's in your PATH: `dot -V`
3. Restart your terminal/IDE after installation

### Images not appearing

Ensure the PNG files exist in `docs/_static/`:
- `benchmark_comparison.png`
- `benchmark_scatter.png`

If missing, run the visualization script as described above.
