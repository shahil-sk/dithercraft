---
name: dither-ui
description: >-
  Native desktop UI architecture, egui/eframe controls, and non-blocking Rayon threading for Dither Guy.
  Use when developing UI panels, implementing preview canvas shaders, or handling async workers.
---

# Dither Guy (Rust) — Desktop UI Architecture

Guidelines for developing the hardware-accelerated desktop interface in `crates/dither-gui`.

## Technology Stack
- **Framework**: `eframe` / `egui` (immediate-mode graphical user interface).
- **Renderer Backend**: WGPU (Vulkan / Metal / DX12 / OpenGL hardware acceleration).
- **Threading**: Background Rayon worker thread pools with non-blocking channels (`crossbeam-channel` or `std::sync::mpsc`).

## UI Invariants

1. **Non-Blocking Main Thread**:
   - The GUI thread runs at display refresh rate ($\ge 60$ FPS).
   - Heavy dithering, video frame streaming, and batch jobs must run asynchronously on background worker threads.
   - The UI communicates with workers via message channels:
     - GUI $\to$ Worker: `JobRequest { buffer, config }`
     - Worker $\to$ GUI: `JobResponse { rendered_texture_data }`
2. **Instant Cancellation**:
   - When a user adjusts parameters rapidly, discard stale in-flight jobs and cancel active computation immediately.
3. **Interactive Canvas**:
   - Zoom ($10\% - 1600\%$) and pan navigation with nearest-neighbor texture filtering to preserve crisp retro pixel edges.
   - Interactive Before/After split-view comparison slider.
4. **Command Dispatching**:
   - UI panels remain thin: actions dispatch commands to the application controller.
