# DitherCraft (Rust) — Vector Isolation & Shaper Engine (`docs/shaper.md`)

The **Vector Isolation & Shaper Engine** (`dither_core::vector::BlackVectorIsolation`) is DitherCraft's clean-room vectorization subsystem. It extracts the darkest pixels and contours from dithered bitmaps into optimized, scalable vector graphics (SVG).

---

## 1. Architectural Purpose & Use Cases

When dithering continuous-tone images with retro algorithms (such as Atkinson, Floyd-Steinberg, or 1-bit Macintosh), the resulting binary pixel distribution represents crisp structural ink boundaries.

The Shaper engine converts this discrete raster grid into vector geometry tailored for:
1. **Vinyl Cutters & Silhouette/Cricut**: Continuous cutlines without pixel stair-stepping artifacts.
2. **Pen Plotters & AxiDraw**: Crisp line work and boundary strokes for physical ink drawing.
3. **Laser Engravers & CNC Mills**: Closed toolpaths with optimized traversal boundaries.
4. **Textile & Screen Printing**: Pure 1-bit vector positive/negative films.

---

## 2. Mathematical Tracing Pipeline

### 2.1 Luminance Binarization
Pixels are analyzed using the ITU-R BT.601 standard photometric formula:
$$Y = 0.299R + 0.587G + 0.114B$$
Pixels satisfying $Y \le \text{max\_luminance\_threshold}$ (default: 128) are classified as active black vector cells.

### 2.2 Boundary Contour Aggregation
For every active pixel cell at grid coordinate $(x, y)$, the Shaper traces external perimeter segments:
- **Top Edge**: $(x, y) \to (x + 1, y)$ if cell $(x, y - 1)$ is background.
- **Bottom Edge**: $(x, y + 1) \to (x + 1, y + 1)$ if cell $(x, y + 1)$ is background.
- **Left Edge**: $(x, y) \to (x, y + 1)$ if cell $(x - 1, y)$ is background.
- **Right Edge**: $(x + 1, y) \to (x + 1, y + 1)$ if cell $(x + 1, y)$ is background.

Internal shared edges between adjacent black pixels are automatically culled, preventing redundant internal cuts or plotter travel strokes.

### 2.3 Horizontal Segment Consolidation
Consecutive horizontal line spans on the same raster scanline are merged into unified vector segments, reducing SVG file size and decreasing plotter node count by up to 60%.

---

## 3. Interfaces & Usage

### 3.1 Pure Rust API (`crates/dither-core`)
```rust
use dither_core::{ImageBuffer, vector::BlackVectorIsolation};

let img = ImageBuffer::new(800, 600)?;
let isolator = BlackVectorIsolation {
    max_luminance_threshold: 128,
    pixel_size: 1.0,
};

// Generates complete, valid SVG XML document string
let svg_string: String = isolator.trace_to_svg(&img)?;
std::fs::write("output.svg", svg_string)?;
```

### 3.2 Headless CLI (`apps/dithercraft-cli`)
```bash
dithercraft-cli svg --input input.png --output output.svg --threshold 128
```

### 3.3 Model Context Protocol (MCP) Tool
Autonomous agents and scripting environments can call `isolate_vector`:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "isolate_vector",
    "arguments": {
      "input_path": "artwork.png",
      "output_path": "cutline.svg",
      "threshold": 128
    }
  }
}
```

### 3.4 Desktop GUI (`apps/dithercraft`)
- Navigate to **File** $\to$ **Export Black Vector SVG...**.
- Select target file location and export instantaneously.
