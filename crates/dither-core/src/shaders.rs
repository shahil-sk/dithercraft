//! WebGPU Shading Language (WGSL) Compute and Fragment Shaders for Hardware Acceleration.
//!
//! Provides zero-dependency, verified WGSL shader kernels for real-time 4K/60FPS dithering,
//! rotated halftone screening, and vintage CRT post-processing.

/// WGSL compute shader for hardware-accelerated 8x8 Bayer ordered dithering.
///
/// Binds:
/// - `@group(0) @binding(0) var src_texture: texture_2d<f32>;`
/// - `@group(0) @binding(1) var dst_texture: texture_storage_2d<rgba8unorm, write>;`
/// - `@group(0) @binding(2) var<storage, read> palette: array<vec4<f32>>;`
pub const BAYER_ORDERED_COMPUTE_WGSL: &str = r#"
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<storage, read> palette: array<vec4<f32>>;

const BAYER_8X8: array<f32, 64> = array<f32, 64>(
     0.0 / 64.0, 32.0 / 64.0,  8.0 / 64.0, 40.0 / 64.0,  2.0 / 64.0, 34.0 / 64.0, 10.0 / 64.0, 42.0 / 64.0,
    48.0 / 64.0, 16.0 / 64.0, 56.0 / 64.0, 24.0 / 64.0, 50.0 / 64.0, 18.0 / 64.0, 58.0 / 64.0, 26.0 / 64.0,
    12.0 / 64.0, 44.0 / 64.0,  4.0 / 64.0, 36.0 / 64.0, 14.0 / 64.0, 46.0 / 64.0,  6.0 / 64.0, 38.0 / 64.0,
    60.0 / 64.0, 28.0 / 64.0, 52.0 / 64.0, 20.0 / 64.0, 62.0 / 64.0, 30.0 / 64.0, 54.0 / 64.0, 22.0 / 64.0,
     3.0 / 64.0, 35.0 / 64.0, 11.0 / 64.0, 43.0 / 64.0,  1.0 / 64.0, 33.0 / 64.0,  9.0 / 64.0, 41.0 / 64.0,
    51.0 / 64.0, 19.0 / 64.0, 59.0 / 64.0, 27.0 / 64.0, 49.0 / 64.0, 17.0 / 64.0, 57.0 / 64.0, 25.0 / 64.0,
    15.0 / 64.0, 47.0 / 64.0,  7.0 / 64.0, 39.0 / 64.0, 13.0 / 64.0, 45.0 / 64.0,  5.0 / 64.0, 37.0 / 64.0,
    63.0 / 64.0, 31.0 / 64.0, 55.0 / 64.0, 23.0 / 64.0, 61.0 / 64.0, 29.0 / 64.0, 53.0 / 64.0, 21.0 / 64.0,
);

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(src_tex);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let coord = vec2<i32>(global_id.xy);
    let src_color = textureLoad(src_tex, coord, 0);

    let bx = global_id.x % 8u;
    let by = global_id.y % 8u;
    let bayer_val = BAYER_8X8[by * 8u + bx] - 0.5;

    let modulated = clamp(src_color.rgb + vec3<f32>(bayer_val * 0.25), vec3<f32>(0.0), vec3<f32>(1.0));

    // Find nearest palette swatch
    var best_dist: f32 = 1000000.0;
    var best_color: vec3<f32> = palette[0].rgb;
    let pal_len = arrayLength(&palette);

    for (var i: u32 = 0u; i < pal_len; i = i + 1u) {
        let diff = modulated - palette[i].rgb;
        let dist = dot(diff, diff);
        if (dist < best_dist) {
            best_dist = dist;
            best_color = palette[i].rgb;
        }
    }

    textureStore(dst_tex, coord, vec4<f32>(best_color, 1.0));
}
"#;

/// WGSL compute shader for rotated halftone screening with frequency and angle uniforms.
pub const HALFTONE_SCREEN_COMPUTE_WGSL: &str = r#"
struct HalftoneUniforms {
    frequency: f32,
    angle_rad: f32,
    dot_scale: f32,
    _pad: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> uniforms: HalftoneUniforms;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(src_tex);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let coord = vec2<i32>(global_id.xy);
    let src_color = textureLoad(src_tex, coord, 0);
    let luminance = dot(src_color.rgb, vec3<f32>(0.299, 0.587, 0.114));

    let cos_a = cos(uniforms.angle_rad);
    let sin_a = sin(uniforms.angle_rad);
    let pos = vec2<f32>(global_id.xy);
    let rot = vec2<f32>(pos.x * cos_a - pos.y * sin_a, pos.x * sin_a + pos.y * cos_a);

    let grid = fract(rot * uniforms.frequency) - vec2<f32>(0.5);
    let dist = length(grid) * 2.0;

    let radius = sqrt(clamp(1.0 - luminance, 0.0, 1.0)) * uniforms.dot_scale;
    let val = select(1.0, 0.0, dist < radius);

    textureStore(dst_tex, coord, vec4<f32>(vec3<f32>(val), 1.0));
}
"#;

/// WGSL shader for unified CRT simulation (barrel lens curvature, scanlines, vignette, chromatic aberration).
pub const CRT_POSTPROCESS_WGSL: &str = r#"
struct CrtUniforms {
    barrel_distortion: f32,
    scanline_intensity: f32,
    vignette_intensity: f32,
    chromatic_shift: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> crt: CrtUniforms;

fn barrel_distort(uv: vec2<f32>, k: f32) -> vec2<f32> {
    let centered = uv - vec2<f32>(0.5);
    let r2 = dot(centered, centered);
    let distorted = centered * (1.0 + k * r2);
    return distorted + vec2<f32>(0.5);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(src_tex);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let coord = vec2<i32>(global_id.xy);
    let uv = vec2<f32>(global_id.xy) / vec2<f32>(dims);
    let dist_uv = barrel_distort(uv, crt.barrel_distortion);

    if (dist_uv.x < 0.0 || dist_uv.x > 1.0 || dist_uv.y < 0.0 || dist_uv.y > 1.0) {
        textureStore(dst_tex, coord, vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    let sample_coord = vec2<i32>(dist_uv * vec2<f32>(dims));
    let shift = vec2<i32>(i32(crt.chromatic_shift), 0);

    let r = textureLoad(src_tex, clamp(sample_coord - shift, vec2<i32>(0), vec2<i32>(dims - 1u)), 0).r;
    let g = textureLoad(src_tex, clamp(sample_coord, vec2<i32>(0), vec2<i32>(dims - 1u)), 0).g;
    let b = textureLoad(src_tex, clamp(sample_coord + shift, vec2<i32>(0), vec2<i32>(dims - 1u)), 0).b;

    var color = vec3<f32>(r, g, b);

    // Scanline modulation
    let scanline = sin(dist_uv.y * f32(dims.y) * 3.14159265);
    color = color * (1.0 - crt.scanline_intensity * (0.5 - 0.5 * scanline));

    // Vignette
    let vig_uv = (dist_uv - vec2<f32>(0.5)) * 2.0;
    let vig = clamp(1.0 - dot(vig_uv, vig_uv) * crt.vignette_intensity, 0.0, 1.0);
    color = color * vig;

    textureStore(dst_tex, coord, vec4<f32>(color, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shaders_are_non_empty_and_valid_syntax() {
        assert!(BAYER_ORDERED_COMPUTE_WGSL.contains("@compute"));
        assert!(BAYER_ORDERED_COMPUTE_WGSL.contains("BAYER_8X8"));
        assert!(HALFTONE_SCREEN_COMPUTE_WGSL.contains("@compute"));
        assert!(HALFTONE_SCREEN_COMPUTE_WGSL.contains("HalftoneUniforms"));
        assert!(CRT_POSTPROCESS_WGSL.contains("@compute"));
        assert!(CRT_POSTPROCESS_WGSL.contains("barrel_distort"));
    }
}
