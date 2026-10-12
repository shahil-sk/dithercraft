# DitherCraft (Rust) — Vector Isolation & Shaper Engine (`docs/shaper.md`)

**Document Version:** 3.0.0  
**Last Updated:** October 12, 2026  
**Classification:** Vector Geometry & Toolpath Generation Architecture  

The **Vector Isolation & Shaper Engine** (`dither_core::vector`) is DitherCraft's clean-room vectorization and CNC toolpath subsystem. It extracts pixel structures and contours from dithered bitmaps into optimized, scalable vector graphics (SVG) and machine-ready G-code toolpaths.

---

## 1. Architectural Purpose & Use Cases

When dithering continuous-tone images with retro algorithms (such as Atkinson, Floyd-Steinberg, or 1-bit Macintosh), the resulting discrete pixel distribution forms crisp graphic boundaries.

The Shaper engine converts this discrete raster grid into vector geometry tailored for:
1. **Pen Plotters & AxiDraw**: Crisp line work, smooth Bézier strokes, and minimized rapid pen-up transit times.
2. **Vinyl Cutters & Silhouette/Cricut**: Continuous, simplified cutlines without pixel stair-stepping artifacts.
3. **Laser Engravers & CNC Mills**: Clean toolpaths (`G00` travel, `G01` cut) with Traveling Salesperson (TSP) path sorting.
4. **Textile & Screen Printing**: Multi-layer color separations isolating each palette swatch into dedicated SVG groups.

---

## 2. Geometry & Simplification Pipeline

```text
[Dithered ImageBuffer]
        │
        ├──► [Black Vector Isolation] ──► [Perimeter Contours]
        │                                         │
        │                        ┌────────────────┴────────────────┐
        │                        ▼                                 ▼
        │               [RDP Simplification]            [Visvalingam-Whyatt]
        │                        │                                 │
        │                        └────────────────┬────────────────┘
        │                                         ▼
        │                              [Cubic Bézier Fitting]
        │                                         │
        ├──► [Multi-Color Separation] ────────────┼──► [Multi-Layer SVG]
        │                                         ▼
        └──► [Toolpath Optimizer (TSP)] ────────────────► [Optimized G-Code]
```

### 2.1 Luminance Binarization & Contour Aggregation
Pixels are analyzed using the ITU-R BT.601 standard photometric formula:
$$Y = 0.299R + 0.587G + 0.114B$$
Pixels satisfying $Y \le \text{max\_luminance\_threshold}$ are classified as active black vector cells. External boundary segments are traced while internal shared edges between adjacent pixels are culled.

### 2.2 Ramer-Douglas-Peucker (RDP) Simplification (`rdp_simplify`)
- **Mathematical Grounding**: Ramer (1972) and Douglas & Peucker (1973).
- Evaluates perpendicular distances between line segments and intermediate vertices. Vertices within distance tolerance $\epsilon$ are discarded while retaining sharp corners and apexes.
- **Node Reduction**: Reduces SVG node count by $70\%\text{--}90\%$ with zero loss of silhouette legibility.

### 2.3 Visvalingam-Whyatt Simplification (`visvalingam_simplify`)
- **Mathematical Grounding**: Visvalingam & Whyatt (1993).
- Iteratively computes the triangular area formed by consecutive triplets of vertices $(P_{i-1}, P_i, P_{i+1})$, removing the vertex with the smallest effective area until an area threshold is met.
- **Visual Characteristic**: Preserves smooth, organic curves without introducing angular spikes or flat facets.

### 2.4 Piecewise Cubic Bézier Curve Fitting (`fit_cubic_spline`, `export_smooth_bezier_svg`)
- **Mathematical Grounding**: Schneider (1990).
- Fits $C^1$-continuous cubic Bézier curves over discrete polyline vertices:
  $$\mathbf{B}(t) = (1-t)^3 \mathbf{P}_0 + 3(1-t)^2 t \mathbf{P}_1 + 3(1-t) t^2 \mathbf{P}_2 + t^3 \mathbf{P}_3, \quad t \in [0, 1]$$
- Generates smooth `<path d="M... C...">` vector commands, eliminating stairstep jitter on physical pen plotters.

---

## 3. Multi-Color Separation Layers (`MultiColorSvgSeparation`)

For multi-color dithered artwork, the Shaper engine automatically groups vector paths by palette swatch:
- Scans each pixel coordinate and identifies its quantized palette swatch index.
- Groups connected contours and exports an SVG containing dedicated `<g id="color-{i}" fill="{hex}">` groups.
- Directly compatible with multi-pen AxiDraw plotters and multi-screen silkscreen printing.

---

## 4. CNC / Plotter G-Code Generation (`GCodeExporter`)

The G-code exporter generates standardized machine instructions for CNC plotters, laser engravers, and vinyl cutters:
- **`G00`**: Rapid traverse with pen-up (`Z_UP` coordinate).
- **`G01`**: Linear cut motion with pen-down (`Z_DOWN` coordinate) and configurable feedrate ($F$).
- **Spindle / Laser Modulation**: `M03 S{power}` and `M05` commands.
- **Pen-Lift Delays**: Configurable dwell times (`G04 P{ms}`) preventing ink bleed at start points.

### Traveling Salesperson (TSP) Path Optimization
Naive vector exports cause pen plotters to spend $> 50\%$ of drawing time making rapid pen-up transit moves across the canvas.
- DitherCraft applies a nearest-neighbor spatial sorting algorithm across path starting and ending points.
- Paths are automatically reversed when ending closer to the current tool position, minimizing rapid travel distance by up to **60%**.

---

## 5. Interfaces & Usage

### 5.1 Pure Rust API (`crates/dither-core`)
```rust
use dither_core::{
    ImageBuffer,
    vector::{
        BlackVectorIsolation, GCodeConfig, GCodeExporter, MultiColorSvgSeparation,
        export_smooth_bezier_svg, rdp_simplify,
    },
};

// 1. Black vector contour tracing
let img = ImageBuffer::new(800, 600)?;
let isolator = BlackVectorIsolation::default();
let raw_svg: String = isolator.trace_to_svg(&img)?;

// 2. Smooth cubic Bézier SVG export
let bezier_svg: String = export_smooth_bezier_svg(&img, 1.5)?;

// 3. Multi-color layer separation SVG
let separator = MultiColorSvgSeparation::default();
let layered_svg: String = separator.export_layered_svg(&img, &palette)?;

// 4. Optimized G-Code export
let gcode_cfg = GCodeConfig::default();
let gcode_exporter = GCodeExporter::new(gcode_cfg);
let gcode_text: String = gcode_exporter.generate_gcode(&img)?;
```

### 5.2 Desktop Workstation (`apps/dithercraft`)
- **File** $\to$ **Export Black Vector SVG...**: Standard single-layer contour SVG.
- **File** $\to$ **Export Multi-Color SVG...**: Layered SVG with `<g id="color-...">` per swatch.
- **File** $\to$ **Export Smooth Bézier SVG...**: Piecewise cubic spline vector path.
- **File** $\to$ **Export CNC Plotter G-Code...**: Machine-ready G-code with TSP optimization.

### 5.3 Headless CLI (`apps/dithercraft-cli`)
```bash
# Black vector isolation
dithercraft-cli svg --input input.png --output cutline.svg --threshold 128

# Multi-color SVG separation
dithercraft-cli layered-svg --input input.png --output multi.svg

# G-Code plotter generation
dithercraft-cli gcode --input input.png --output toolpath.gcode --feedrate 3000
```

---

## 6. Next-Gen Roadmap Vectors (Phase 9)

In upcoming development (Phase 9):
1. **Multi-Pass Tool Offsets**: Automatic path dilation and erosion offsets compensating for laser kerf width and vinyl cutter blade offset radius.
2. **Configurable Crosshatch Fills**: Interior polygon hatching with adjustable line angles ($0^\circ, 45^\circ, 90^\circ$) and line spacing for solid area filling with fine-tip pens.
