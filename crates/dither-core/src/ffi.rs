//! Stable C/Rust DSP Plugin FFI ABI (`dither_core_ffi`).
//!
//! Exposes a clean, zero-allocation C ABI for third-party dynamic libraries (.so, .dylib, .dll),
//! WebAssembly host environments, and external language bindings (C, C++, Python, Zig).

#![allow(unsafe_code, clippy::not_unsafe_ptr_arg_deref)]

use crate::algorithms::create_algorithm;
use crate::buffer::ImageBuffer;
use crate::color::RgbColor;
use crate::palette::Palette;
use std::ffi::{CStr, c_char};

pub const DITHER_OK: i32 = 0;
pub const DITHER_ERR_NULL_PTR: i32 = -1;
pub const DITHER_ERR_INVALID_PARAM: i32 = -2;
pub const DITHER_ERR_ALGO_NOT_FOUND: i32 = -3;
pub const DITHER_ERR_EXECUTION_FAILED: i32 = -4;

const VERSION_C_STR: &[u8] = b"0.9.0\0";

/// Return static C-string of DitherCraft Core engine version.
#[unsafe(no_mangle)]
pub extern "C" fn dither_version() -> *const c_char {
    VERSION_C_STR.as_ptr() as *const c_char
}

/// Allocate a new heap ImageBuffer with width and height.
#[unsafe(no_mangle)]
pub extern "C" fn dither_create_image(width: u32, height: u32) -> *mut ImageBuffer {
    match ImageBuffer::new(width, height) {
        Ok(buf) => Box::into_raw(Box::new(buf)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free an ImageBuffer handle previously created via `dither_create_image`.
#[unsafe(no_mangle)]
pub extern "C" fn dither_destroy_image(handle: *mut ImageBuffer) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Retrieve image dimensions.
#[unsafe(no_mangle)]
pub extern "C" fn dither_image_get_dimensions(handle: *const ImageBuffer, out_width: *mut u32, out_height: *mut u32) -> i32 {
    if handle.is_null() || out_width.is_null() || out_height.is_null() {
        return DITHER_ERR_NULL_PTR;
    }
    unsafe {
        *out_width = (*handle).width;
        *out_height = (*handle).height;
    }
    DITHER_OK
}

/// Retrieve pointer to raw contiguous RGB24 pixel buffer data.
#[unsafe(no_mangle)]
pub extern "C" fn dither_image_get_data(handle: *mut ImageBuffer) -> *mut u8 {
    if handle.is_null() { std::ptr::null_mut() } else { unsafe { (*handle).data.as_mut_ptr() } }
}

/// Retrieve total byte length of raw contiguous RGB24 pixel buffer data.
#[unsafe(no_mangle)]
pub extern "C" fn dither_image_get_data_len(handle: *const ImageBuffer) -> usize {
    if handle.is_null() { 0 } else { unsafe { (*handle).data.len() } }
}

/// Create a new Palette from packed RGB byte slice (`[R0, G0, B0, R1, G1, B1, ...]`).
#[unsafe(no_mangle)]
pub extern "C" fn dither_create_palette(num_colors: u32, rgb_bytes: *const u8) -> *mut Palette {
    if rgb_bytes.is_null() || num_colors == 0 {
        return std::ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(rgb_bytes, (num_colors as usize) * 3) };
    let colors: Vec<RgbColor> = slice.chunks_exact(3).map(|chunk| RgbColor::new(chunk[0], chunk[1], chunk[2])).collect();

    match Palette::new("FFI Palette", colors) {
        Ok(pal) => Box::into_raw(Box::new(pal)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a Palette handle previously created via `dither_create_palette`.
#[unsafe(no_mangle)]
pub extern "C" fn dither_destroy_palette(palette: *mut Palette) {
    if !palette.is_null() {
        unsafe {
            drop(Box::from_raw(palette));
        }
    }
}

/// Process image through a named dithering algorithm writing to destination image buffer.
#[unsafe(no_mangle)]
pub extern "C" fn dither_process_image(
    algo_name: *const c_char,
    input: *const ImageBuffer,
    palette: *const Palette,
    output: *mut ImageBuffer,
) -> i32 {
    if algo_name.is_null() || input.is_null() || palette.is_null() || output.is_null() {
        return DITHER_ERR_NULL_PTR;
    }

    let name_str = match unsafe { CStr::from_ptr(algo_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return DITHER_ERR_INVALID_PARAM,
    };

    let algo = match create_algorithm(name_str) {
        Some(a) => a,
        None => return DITHER_ERR_ALGO_NOT_FOUND,
    };

    let in_ref = unsafe { &*input };
    let pal_ref = unsafe { &*palette };
    let out_ref = unsafe { &mut *output };

    match algo.apply(in_ref, pal_ref) {
        Ok(res) => {
            if res.width != out_ref.width || res.height != out_ref.height {
                *out_ref = res;
            } else {
                out_ref.data.copy_from_slice(&res.data);
            }
            DITHER_OK
        }
        Err(_) => DITHER_ERR_EXECUTION_FAILED,
    }
}

/// Execute canvas geometry operations (1=Rotate 90 CW, 2=Rotate 180, 3=Rotate 270 CW, 4=Flip H, 5=Flip V, 6=Scale Integer).
#[unsafe(no_mangle)]
pub extern "C" fn dither_transform_image(op: i32, param: u32, input: *const ImageBuffer, output: *mut ImageBuffer) -> i32 {
    if input.is_null() || output.is_null() {
        return DITHER_ERR_NULL_PTR;
    }

    let in_ref = unsafe { &*input };
    let out_ref = unsafe { &mut *output };

    let transformed = match op {
        1 => in_ref.rotate_90_cw(),
        2 => in_ref.rotate_180(),
        3 => in_ref.rotate_270_cw(),
        4 => in_ref.flip_horizontal(),
        5 => in_ref.flip_vertical(),
        6 => in_ref.scale_integer(param),
        _ => return DITHER_ERR_INVALID_PARAM,
    };

    match transformed {
        Ok(res) => {
            *out_ref = res;
            DITHER_OK
        }
        Err(_) => DITHER_ERR_EXECUTION_FAILED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_c_ffi_lifecycle() {
        let v_ptr = dither_version();
        assert!(!v_ptr.is_null());
        let v_str = unsafe { CStr::from_ptr(v_ptr) }.to_str().expect("version str");
        assert_eq!(v_str, "0.9.0");

        let img = dither_create_image(16, 16);
        assert!(!img.is_null());

        let mut w = 0;
        let mut h = 0;
        let res = dither_image_get_dimensions(img, &mut w, &mut h);
        assert_eq!(res, DITHER_OK);
        assert_eq!(w, 16);
        assert_eq!(h, 16);

        let data = dither_image_get_data(img);
        assert!(!data.is_null());
        let len = dither_image_get_data_len(img);
        assert_eq!(len, 16 * 16 * 3);

        let rgb = [0u8, 0, 0, 255, 255, 255];
        let pal = dither_create_palette(2, rgb.as_ptr());
        assert!(!pal.is_null());

        let out_img = dither_create_image(16, 16);
        let name = CString::new("bayer8x8").expect("name");
        let proc_res = dither_process_image(name.as_ptr(), img, pal, out_img);
        assert_eq!(proc_res, DITHER_OK);

        let rot_res = dither_transform_image(1, 0, img, out_img);
        assert_eq!(rot_res, DITHER_OK);

        dither_destroy_palette(pal);
        dither_destroy_image(img);
        dither_destroy_image(out_img);
    }
}
