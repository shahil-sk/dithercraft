/**
 * DitherCraft Core — Stable C/Rust DSP Plugin ABI Header
 * Version: 0.9.0
 * 
 * Clean-room Pure Rust DSP Engine for Retro Dithering & Shaders.
 * Compatible with C99, C++, WebAssembly (Emscripten / wasm32), Python ctypes, Zig.
 */

#ifndef DITHER_CORE_FFI_H
#define DITHER_CORE_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status Codes */
#define DITHER_OK                      0
#define DITHER_ERR_NULL_PTR           -1
#define DITHER_ERR_INVALID_PARAM      -2
#define DITHER_ERR_ALGO_NOT_FOUND     -3
#define DITHER_ERR_EXECUTION_FAILED   -4

/* Canvas Geometry Operations */
#define DITHER_GEOMETRY_ROTATE_90_CW   1
#define DITHER_GEOMETRY_ROTATE_180     2
#define DITHER_GEOMETRY_ROTATE_270_CW  3
#define DITHER_GEOMETRY_FLIP_H         4
#define DITHER_GEOMETRY_FLIP_V         5
#define DITHER_GEOMETRY_SCALE_INTEGER  6

/* Opaque Handles */
typedef struct ImageBuffer ImageBuffer;
typedef struct Palette Palette;

/**
 * Returns static null-terminated version string of DitherCraft Core engine.
 */
const char* dither_version(void);

/**
 * Allocates a new ImageBuffer with dimensions `width` x `height`.
 * Returns NULL on allocation error or invalid parameters.
 */
ImageBuffer* dither_create_image(uint32_t width, uint32_t height);

/**
 * Frees an ImageBuffer previously allocated via `dither_create_image`.
 */
void dither_destroy_image(ImageBuffer* handle);

/**
 * Retrieves the width and height of an ImageBuffer.
 */
int32_t dither_image_get_dimensions(const ImageBuffer* handle, uint32_t* out_width, uint32_t* out_height);

/**
 * Returns pointer to contiguous raw 24-bit RGB pixel data (RGB888).
 */
uint8_t* dither_image_get_data(ImageBuffer* handle);

/**
 * Returns total byte length of the raw 24-bit RGB pixel data buffer.
 */
size_t dither_image_get_data_len(const ImageBuffer* handle);

/**
 * Creates a new Palette from packed RGB bytes `[R0, G0, B0, R1, G1, B1, ...]`.
 * `num_colors` is the total count of colors (array size is `num_colors * 3`).
 */
Palette* dither_create_palette(uint32_t num_colors, const uint8_t* rgb_bytes);

/**
 * Frees a Palette previously allocated via `dither_create_palette`.
 */
void dither_destroy_palette(Palette* palette);

/**
 * Processes an input image buffer through a named dithering algorithm
 * writing the quantized result into `output`.
 * 
 * Algorithm names include: "floyd-steinberg", "atkinson", "bayer8x8", "bayer4x4",
 * "riemersma", "sierra3", "burkes", "void-and-cluster", "blue-noise-fm", "cmyk",
 * "wavefront", "morphogenesis", "flow-field", etc.
 */
int32_t dither_process_image(
    const char* algo_name,
    const ImageBuffer* input,
    const Palette* palette,
    ImageBuffer* output
);

/**
 * Executes a canvas geometry operation on `input`, writing result to `output`.
 * `op`: One of DITHER_GEOMETRY_* constants.
 * `param`: Additional parameter (e.g. scale factor for integer scaling).
 */
int32_t dither_transform_image(
    int32_t op,
    uint32_t param,
    const ImageBuffer* input,
    ImageBuffer* output
);

#ifdef __cplusplus
}
#endif

#endif /* DITHER_CORE_FFI_H */
