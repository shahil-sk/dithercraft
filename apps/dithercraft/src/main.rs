#![allow(clippy::chunks_exact_to_as_chunks)]

use dither_core::{
    AudioSpectrum, AudioTrack, CadenceMode, CmykHalftone, Cube3dLut, FlowFieldConfig, FlowFieldSteeredDither, HalftoneDotShape, ImageBuffer,
    Mp4Demuxer, OnionSkinConfig, ReactionDiffusionConfig, ReactionDiffusionDither, ReactionDiffusionMorphology, TonalBandDither, WebmDemuxer,
    WebpAnimFrame, WebpAnimationMuxer,
    algorithms::*,
    color::{ColorHarmony, CvdMode, RgbColor},
    effects::*,
    mask::MaskBuffer,
    palette::{ColorMetric, Palette, Swatch},
    selection::{SelectionBuffer, SelectionMode},
    temporal::{TemporalCoherenceConfig, TemporalCoherenceFilter, TemporalNoisePattern, render_onion_skin},
    vector::{BlackVectorIsolation, GCodeConfig, GCodeExporter, MultiColorSvgSeparation, SpotColorPlateSeparation, export_smooth_bezier_svg},
};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::{AnimationDecoder, Frame, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

/// Active interface theme out of the 5 studio themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioTheme {
    StudioDark,
    AmberCrt,
    CyberMatrix,
    VaporSlate,
    IndustrialHighContrast,
}

impl StudioTheme {
    pub fn name(&self) -> &'static str {
        match self {
            Self::StudioDark => "Studio Dark",
            Self::AmberCrt => "Amber CRT",
            Self::CyberMatrix => "Cyber Matrix",
            Self::VaporSlate => "Vapor Slate",
            Self::IndustrialHighContrast => "Industrial High-Contrast",
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        match self {
            Self::StudioDark => {
                style.visuals = egui::Visuals::dark();
            }
            Self::AmberCrt => {
                let mut v = egui::Visuals::dark();
                v.override_text_color = Some(Color32::from_rgb(255, 176, 0));
                v.panel_fill = Color32::from_rgb(20, 14, 0);
                v.window_fill = Color32::from_rgb(28, 18, 0);
                style.visuals = v;
            }
            Self::CyberMatrix => {
                let mut v = egui::Visuals::dark();
                v.override_text_color = Some(Color32::from_rgb(0, 255, 65));
                v.panel_fill = Color32::from_rgb(5, 12, 5);
                v.window_fill = Color32::from_rgb(10, 20, 10);
                style.visuals = v;
            }
            Self::VaporSlate => {
                let mut v = egui::Visuals::dark();
                v.override_text_color = Some(Color32::from_rgb(255, 113, 206));
                v.panel_fill = Color32::from_rgb(25, 20, 36);
                v.window_fill = Color32::from_rgb(38, 30, 56);
                style.visuals = v;
            }
            Self::IndustrialHighContrast => {
                let mut v = egui::Visuals::light();
                v.override_text_color = Some(Color32::BLACK);
                v.panel_fill = Color32::from_rgb(240, 240, 240);
                style.visuals = v;
            }
        }
        ctx.set_style(style);
    }
}

/// Viewport comparison mode between source image and dithered output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitViewMode {
    #[default]
    Vertical,
    Horizontal,
    Checkerboard,
    Difference,
}

impl SplitViewMode {
    pub const ALL: [Self; 4] = [Self::Vertical, Self::Horizontal, Self::Checkerboard, Self::Difference];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Vertical => "Vertical Split",
            Self::Horizontal => "Horizontal Split",
            Self::Checkerboard => "Checkerboard A/B",
            Self::Difference => "Difference Heatmap",
        }
    }
}

/// Retro Aspect Ratio Framing Guides for pixel-authentic composition preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RetroAspectGuide {
    #[default]
    None,
    GameBoy,     // 10:9 (160x144)
    NesSnes,     // 8:7 (256x224 active)
    StandardCrt, // 4:3 (320x240 / standard CRT)
    ArcadeTate,  // 3:4 (vertical arcade)
    Square,      // 1:1
    Widescreen,  // 16:9
}

impl RetroAspectGuide {
    pub const ALL: [Self; 7] = [Self::None, Self::GameBoy, Self::NesSnes, Self::StandardCrt, Self::ArcadeTate, Self::Square, Self::Widescreen];

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None (Full Frame)",
            Self::GameBoy => "Game Boy (10:9)",
            Self::NesSnes => "NES / SNES (8:7)",
            Self::StandardCrt => "Standard CRT (4:3)",
            Self::ArcadeTate => "Arcade Tate (3:4)",
            Self::Square => "Square (1:1)",
            Self::Widescreen => "Widescreen (16:9)",
        }
    }

    pub fn aspect_ratio(&self) -> Option<f32> {
        match self {
            Self::None => None,
            Self::GameBoy => Some(10.0 / 9.0),
            Self::NesSnes => Some(8.0 / 7.0),
            Self::StandardCrt => Some(4.0 / 3.0),
            Self::ArcadeTate => Some(3.0 / 4.0),
            Self::Square => Some(1.0),
            Self::Widescreen => Some(16.0 / 9.0),
        }
    }
}

/// Active tool selected in the workstation canvas palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkstationTool {
    #[default]
    Pointer,
    BrushMask,
    EraserMask,
    RectMarquee,
    EllipseMarquee,
    Lasso,
    MagicWand,
}

impl WorkstationTool {
    pub const ALL: [Self; 7] =
        [Self::Pointer, Self::BrushMask, Self::EraserMask, Self::RectMarquee, Self::EllipseMarquee, Self::Lasso, Self::MagicWand];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Pointer => "↖ Pointer (V)",
            Self::BrushMask => "🖌 Brush Mask (B)",
            Self::EraserMask => "🧹 Eraser (E)",
            Self::RectMarquee => "⬚ Rect Marquee (M)",
            Self::EllipseMarquee => "⬭ Ellipse Marquee",
            Self::Lasso => "➿ Lasso Marquee",
            Self::MagicWand => "🪄 Magic Wand (W)",
        }
    }
}

fn default_levels_black() -> u8 {
    0
}
fn default_levels_white() -> u8 {
    255
}
fn default_levels_gamma() -> f32 {
    1.0
}
fn default_lut_preset() -> String {
    "Technicolor 2-Strip".to_string()
}

/// Workstation snapshot capturing complete algorithm, palette, and effects configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudioPreset {
    pub name: String,
    pub algo: String,
    pub palette_name: String,
    pub metric: ColorMetric,
    pub tonal_cycle_step: usize,

    #[serde(default = "default_levels_black")]
    pub levels_black: u8,
    #[serde(default = "default_levels_white")]
    pub levels_white: u8,
    #[serde(default = "default_levels_gamma")]
    pub levels_gamma: f32,
    #[serde(default)]
    pub cvd_mode: CvdMode,

    #[serde(default)]
    pub lut_enabled: bool,
    #[serde(default = "default_lut_preset")]
    pub lut_preset: String,

    pub glow_enabled: bool,
    pub glow_threshold: u8,
    pub glow_radius: usize,
    pub glow_intensity: f32,

    pub chromatic_enabled: bool,
    pub chromatic_shift: i32,
    pub chromatic_radial: bool,

    pub glitch_enabled: bool,
    pub glitch_mode: JpegSubMode,
    pub glitch_intensity: f32,

    pub crt_enabled: bool,
    pub crt_scanline: f32,
    pub crt_barrel: f32,
    pub crt_vignette: f32,

    pub blur_enabled: bool,
    pub blur_sigma: f32,

    pub sharpen_enabled: bool,
    pub sharpen_amount: f32,

    pub temporal_enabled: bool,
    pub temporal_pattern: TemporalNoisePattern,
    pub temporal_intensity: f32,
}

impl StudioPreset {
    pub fn bundled_presets() -> Vec<StudioPreset> {
        vec![
            StudioPreset {
                name: "Game Boy Nostalgia".into(),
                algo: "Floyd-Steinberg".into(),
                palette_name: "Game Boy".into(),
                metric: ColorMetric::Redmean,
                tonal_cycle_step: 0,
                levels_black: 0,
                levels_white: 255,
                levels_gamma: 1.0,
                cvd_mode: CvdMode::Normal,
                lut_enabled: false,
                lut_preset: "Technicolor 2-Strip".into(),
                glow_enabled: false,
                glow_threshold: 180,
                glow_radius: 4,
                glow_intensity: 0.8,
                chromatic_enabled: false,
                chromatic_shift: 0,
                chromatic_radial: false,
                glitch_enabled: false,
                glitch_mode: JpegSubMode::BlockShift,
                glitch_intensity: 0.4,
                crt_enabled: true,
                crt_scanline: 0.20,
                crt_barrel: 0.05,
                crt_vignette: 0.25,
                blur_enabled: false,
                blur_sigma: 1.0,
                sharpen_enabled: false,
                sharpen_amount: 0.8,
                temporal_enabled: false,
                temporal_pattern: TemporalNoisePattern::CrtScanBeamJitter,
                temporal_intensity: 0.5,
            },
            StudioPreset {
                name: "Cyberpunk Neon".into(),
                algo: "Stucki".into(),
                palette_name: "Cyberpunk".into(),
                metric: ColorMetric::Oklab,
                tonal_cycle_step: 0,
                levels_black: 0,
                levels_white: 255,
                levels_gamma: 1.0,
                cvd_mode: CvdMode::Normal,
                lut_enabled: true,
                lut_preset: "Cyber Matrix".into(),
                glow_enabled: true,
                glow_threshold: 160,
                glow_radius: 6,
                glow_intensity: 0.9,
                chromatic_enabled: true,
                chromatic_shift: 4,
                chromatic_radial: true,
                glitch_enabled: false,
                glitch_mode: JpegSubMode::BlockShift,
                glitch_intensity: 0.4,
                crt_enabled: false,
                crt_scanline: 0.0,
                crt_barrel: 0.0,
                crt_vignette: 0.0,
                blur_enabled: false,
                blur_sigma: 1.0,
                sharpen_enabled: true,
                sharpen_amount: 1.0,
                temporal_enabled: false,
                temporal_pattern: TemporalNoisePattern::PhosphorGrainHum,
                temporal_intensity: 0.5,
            },
            StudioPreset {
                name: "1980s VHS Broadcast".into(),
                algo: "Atkinson".into(),
                palette_name: "Commodore 64".into(),
                metric: ColorMetric::Ciede2000,
                tonal_cycle_step: 0,
                levels_black: 0,
                levels_white: 255,
                levels_gamma: 1.0,
                cvd_mode: CvdMode::Normal,
                lut_enabled: true,
                lut_preset: "Vintage Warm 1970".into(),
                glow_enabled: false,
                glow_threshold: 180,
                glow_radius: 4,
                glow_intensity: 0.8,
                chromatic_enabled: true,
                chromatic_shift: 3,
                chromatic_radial: false,
                glitch_enabled: false,
                glitch_mode: JpegSubMode::BlockShift,
                glitch_intensity: 0.4,
                crt_enabled: true,
                crt_scanline: 0.35,
                crt_barrel: 0.10,
                crt_vignette: 0.40,
                blur_enabled: false,
                blur_sigma: 1.0,
                sharpen_enabled: false,
                sharpen_amount: 0.8,
                temporal_enabled: true,
                temporal_pattern: TemporalNoisePattern::VhsTapePhaseNoise,
                temporal_intensity: 0.40,
            },
            StudioPreset {
                name: "Newspaper Halftone".into(),
                algo: "Bayer 8x8".into(),
                palette_name: "Monochrome".into(),
                metric: ColorMetric::Oklab,
                tonal_cycle_step: 0,
                levels_black: 0,
                levels_white: 255,
                levels_gamma: 1.0,
                cvd_mode: CvdMode::Normal,
                lut_enabled: false,
                lut_preset: "Technicolor 2-Strip".into(),
                glow_enabled: false,
                glow_threshold: 180,
                glow_radius: 4,
                glow_intensity: 0.8,
                chromatic_enabled: false,
                chromatic_shift: 0,
                chromatic_radial: false,
                glitch_enabled: false,
                glitch_mode: JpegSubMode::BlockShift,
                glitch_intensity: 0.4,
                crt_enabled: false,
                crt_scanline: 0.0,
                crt_barrel: 0.0,
                crt_vignette: 0.0,
                blur_enabled: false,
                blur_sigma: 1.0,
                sharpen_enabled: true,
                sharpen_amount: 1.2,
                temporal_enabled: false,
                temporal_pattern: TemporalNoisePattern::CrtScanBeamJitter,
                temporal_intensity: 0.5,
            },
            StudioPreset {
                name: "Macintosh 1984 Classic".into(),
                algo: "Floyd-Steinberg".into(),
                palette_name: "Macintosh 1-bit".into(),
                metric: ColorMetric::EuclideanRgb,
                tonal_cycle_step: 0,
                levels_black: 0,
                levels_white: 255,
                levels_gamma: 1.0,
                cvd_mode: CvdMode::Normal,
                lut_enabled: false,
                lut_preset: "Technicolor 2-Strip".into(),
                glow_enabled: false,
                glow_threshold: 180,
                glow_radius: 4,
                glow_intensity: 0.8,
                chromatic_enabled: false,
                chromatic_shift: 0,
                chromatic_radial: false,
                glitch_enabled: false,
                glitch_mode: JpegSubMode::BlockShift,
                glitch_intensity: 0.4,
                crt_enabled: true,
                crt_scanline: 0.15,
                crt_barrel: 0.04,
                crt_vignette: 0.20,
                blur_enabled: false,
                blur_sigma: 1.0,
                sharpen_enabled: false,
                sharpen_amount: 0.8,
                temporal_enabled: false,
                temporal_pattern: TemporalNoisePattern::CrtScanBeamJitter,
                temporal_intensity: 0.5,
            },
        ]
    }
}

struct RenderJobResult {
    buffer: ImageBuffer,
    duration_ms: f64,
    dither_ms: f64,
    effects_ms: f64,
    histogram_ms: f64,
    gen_id: u64,
    swatch_counts: Vec<usize>,
}

pub struct DitherCraftApp {
    // Current input image and rendered result
    source_image: Option<Arc<ImageBuffer>>,
    rendered_image: Option<ImageBuffer>,
    texture: Option<TextureHandle>,

    // Processing configuration
    selected_algo: String,
    algo_search_query: String,
    selected_palette_name: String,
    active_palette: Palette,
    tonal_cycle_step: usize,

    // Effects
    glow_enabled: bool,
    glow_threshold: u8,
    glow_radius: usize,
    glow_intensity: f32,

    chromatic_enabled: bool,
    chromatic_shift: i32,
    chromatic_radial: bool,

    glitch_enabled: bool,
    glitch_mode: JpegSubMode,
    glitch_intensity: f32,

    crt_enabled: bool,
    crt_scanline: f32,
    crt_barrel: f32,
    crt_vignette: f32,

    blur_enabled: bool,
    blur_sigma: f32,

    sharpen_enabled: bool,
    sharpen_amount: f32,

    temporal_noise_enabled: bool,
    temporal_pattern: TemporalNoisePattern,
    temporal_intensity: f32,
    frame_counter: u32,

    // UI & Viewport
    theme: StudioTheme,
    debounce_mode: bool,
    status_text: String,
    render_latency_ms: f64,
    telemetry_dither_ms: f64,
    telemetry_effects_ms: f64,
    telemetry_histogram_ms: f64,
    show_telemetry_hud: bool,

    // Viewport Split-View
    split_view_enabled: bool,
    split_mode: SplitViewMode,
    split_position: f32,
    split_checker_tile_size: usize,

    // Timeline Editor & Transport
    is_playing: bool,
    target_fps: u32,
    loop_mode: bool,
    timeline_frame: u32,
    timeline_max_frames: u32,
    last_frame_instant: Instant,
    source_frames: Vec<Arc<ImageBuffer>>,
    frame_delays: Vec<u32>,
    fine_wheel_increment: bool,

    // App Brand Logo & About Dialog
    logo_texture: Option<TextureHandle>,
    show_about: bool,

    // Interactive Canvas Ergonomics
    canvas_pan: egui::Vec2,
    canvas_zoom: f32,
    show_pixel_grid: bool,

    // Swatch utilization statistics
    swatch_percentages: Vec<f32>,

    // Command History & Undo/Redo
    undo_stack: Vec<StudioPreset>,
    redo_stack: Vec<StudioPreset>,

    // Threading, cancellation & UI wakeups
    render_generation: Arc<AtomicU64>,
    egui_ctx: egui::Context,
    tx: Sender<RenderJobResult>,
    rx: Receiver<RenderJobResult>,
    is_rendering: bool,

    // Batch Processing Studio
    show_batch_studio: bool,
    batch_input_dir: String,
    batch_output_dir: String,
    batch_status_msg: String,
    batch_progress: f32,
    batch_is_running: bool,

    // Audio-Reactive DSP
    audio_track: Option<AudioTrack>,
    audio_spectrum: AudioSpectrum,
    audio_reactive_enabled: bool,

    // File tracking
    source_path: Option<PathBuf>,
    recent_files: Vec<PathBuf>,

    // Phase 8 Workstation Features
    snapshots: [Option<StudioPreset>; 4],
    show_loupe: bool,
    show_shortcuts_dialog: bool,
    levels_black: u8,
    levels_white: u8,
    levels_gamma: f32,
    cvd_mode: CvdMode,
    aspect_guide: RetroAspectGuide,
    show_preset_gallery: bool,

    // Phase 8-10 Studio Features: OKLCH Harmonies & 3D LUT
    show_harmonies_dialog: bool,
    harmony_base_rgb: [u8; 3],
    harmony_mode: ColorHarmony,
    harmony_count: usize,
    lut_enabled: bool,
    lut_preset_name: String,
    custom_lut: Option<Cube3dLut>,

    // Phase 10: CMYK Rosette Screen & Separation Plates
    cmyk_dot_shape: HalftoneDotShape,
    cmyk_dot_gain: f32,

    // Phase 9: Selective Tonal Band Multi-Dither
    tonal_shadow_algo: String,
    tonal_mid_algo: String,
    tonal_highlight_algo: String,
    tonal_shadow_threshold: u8,
    tonal_highlight_threshold: u8,
    tonal_feather: u8,

    // Phase 9: Custom Threshold Matrix Workshop
    custom_matrix: CustomMatrixDither,

    // Phase 11: Frame Cadence & Onion Skinning Lightbox
    cadence_mode: CadenceMode,
    onion_skin_enabled: bool,
    onion_skin_config: OnionSkinConfig,

    // Phase 12: Generative Morphogenesis & Flow Field
    rd_morphology: ReactionDiffusionMorphology,
    rd_steps: usize,
    flow_scale: f32,
    flow_swirl: f32,

    // Phase 8.9: Canvas Geometry Workshop
    crop_dialog_open: bool,
    crop_x: u32,
    crop_y: u32,
    crop_width: u32,
    crop_height: u32,

    // Phase 11.2: Inter-Frame Temporal Coherence
    temporal_coherence_enabled: bool,
    temporal_coherence_damping: f32,
    temporal_coherence_threshold: f32,
    last_rendered_source: Option<ImageBuffer>,
    last_rendered_output: Option<ImageBuffer>,

    // Phase 8.2: Draggable & Reorderable Downstream Effects Stack
    effect_stack: EffectStack,

    // Phase 14-16: Tooling, Spatial Masking & Selection Marquees
    active_tool: WorkstationTool,
    brush_radius: f32,
    brush_falloff: f32,
    mask: Option<MaskBuffer>,
    mask_blend_enabled: bool,
    secondary_algo: String,
    quick_mask_active: bool,
    selection: Option<SelectionBuffer>,
    selection_mode: SelectionMode,
    wand_tolerance: f32,
    lasso_points: Vec<(f32, f32)>,
    drag_start_pixel: Option<(f32, f32)>,
    drag_current_pixel: Option<(f32, f32)>,
    marching_ants_phase: f32,
}

impl DitherCraftApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        let render_generation = Arc::new(AtomicU64::new(0));
        let egui_ctx = cc.egui_ctx.clone();

        let logo_texture =
            image::load_from_memory(include_bytes!("../../../assets/app-icon/hicolor/128x128/apps/io.github.dithercraft.png")).ok().map(|img| {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                cc.egui_ctx.load_texture("app_logo", color_image, TextureOptions::LINEAR)
            });

        // Default initial image: 256x256 test radial gradient
        let mut default_img = ImageBuffer::new(256, 256).unwrap_or_else(|_| ImageBuffer { width: 1, height: 1, data: vec![0, 0, 0] });
        for y in 0..256 {
            for x in 0..256 {
                let r = x as u8;
                let g = y as u8;
                let b = ((x + y) / 2) as u8;
                default_img.set_pixel(x, y, RgbColor::new(r, g, b));
            }
        }

        let default_arc = Arc::new(default_img);

        Self {
            source_image: Some(default_arc.clone()),
            source_frames: vec![default_arc],
            frame_delays: vec![100],
            fine_wheel_increment: false,
            rendered_image: None,
            texture: None,
            selected_algo: "Floyd-Steinberg".into(),
            algo_search_query: String::new(),
            selected_palette_name: "Game Boy".into(),
            active_palette: Palette::game_boy(),
            tonal_cycle_step: 0,

            glow_enabled: false,
            glow_threshold: 180,
            glow_radius: 4,
            glow_intensity: 0.8,

            chromatic_enabled: false,
            chromatic_shift: 3,
            chromatic_radial: false,

            glitch_enabled: false,
            glitch_mode: JpegSubMode::BlockShift,
            glitch_intensity: 0.4,

            crt_enabled: false,
            crt_scanline: 0.25,
            crt_barrel: 0.08,
            crt_vignette: 0.3,

            blur_enabled: false,
            blur_sigma: 1.5,

            sharpen_enabled: false,
            sharpen_amount: 0.8,

            temporal_noise_enabled: false,
            temporal_pattern: TemporalNoisePattern::CrtScanBeamJitter,
            temporal_intensity: 0.5,
            frame_counter: 0,

            theme: StudioTheme::StudioDark,
            debounce_mode: false,
            status_text: "Ready".into(),
            render_latency_ms: 0.0,
            telemetry_dither_ms: 0.0,
            telemetry_effects_ms: 0.0,
            telemetry_histogram_ms: 0.0,
            show_telemetry_hud: false,

            split_view_enabled: false,
            split_mode: SplitViewMode::default(),
            split_position: 0.5,
            split_checker_tile_size: 32,

            is_playing: false,
            target_fps: 30,
            loop_mode: true,
            timeline_frame: 0,
            timeline_max_frames: 60,
            last_frame_instant: Instant::now(),

            logo_texture,
            show_about: false,

            canvas_pan: egui::Vec2::ZERO,
            canvas_zoom: 1.0,
            show_pixel_grid: true,

            swatch_percentages: Vec::new(),

            undo_stack: Vec::new(),
            redo_stack: Vec::new(),

            render_generation,
            egui_ctx,
            tx,
            rx,
            is_rendering: false,

            show_batch_studio: false,
            batch_input_dir: String::new(),
            batch_output_dir: String::new(),
            batch_status_msg: "Ready".into(),
            batch_progress: 0.0,
            batch_is_running: false,

            audio_track: None,
            audio_spectrum: AudioSpectrum::default(),
            audio_reactive_enabled: false,

            source_path: None,
            recent_files: Self::load_recent_files(),

            snapshots: [None, None, None, None],
            show_loupe: false,
            show_shortcuts_dialog: false,
            levels_black: 0,
            levels_white: 255,
            levels_gamma: 1.0,
            cvd_mode: CvdMode::Normal,
            aspect_guide: RetroAspectGuide::None,
            show_preset_gallery: false,

            show_harmonies_dialog: false,
            harmony_base_rgb: [255, 90, 30],
            harmony_mode: ColorHarmony::Triadic,
            harmony_count: 6,
            lut_enabled: false,
            lut_preset_name: "Technicolor 2-Strip".into(),
            custom_lut: None,

            cmyk_dot_shape: HalftoneDotShape::EuclideanRound,
            cmyk_dot_gain: 0.0,

            tonal_shadow_algo: "Bayer 8x8".into(),
            tonal_mid_algo: "Floyd-Steinberg".into(),
            tonal_highlight_algo: "Atkinson".into(),
            tonal_shadow_threshold: 85,
            tonal_highlight_threshold: 170,
            tonal_feather: 16,

            custom_matrix: CustomMatrixDither::default(),

            cadence_mode: CadenceMode::default(),
            onion_skin_enabled: false,
            onion_skin_config: OnionSkinConfig::default(),

            rd_morphology: ReactionDiffusionMorphology::default(),
            rd_steps: 60,
            flow_scale: 0.04,
            flow_swirl: 1.5,

            crop_dialog_open: false,
            crop_x: 0,
            crop_y: 0,
            crop_width: 256,
            crop_height: 256,

            temporal_coherence_enabled: false,
            temporal_coherence_damping: 0.65,
            temporal_coherence_threshold: 0.06,
            last_rendered_source: None,
            last_rendered_output: None,

            effect_stack: EffectStack::default_stack(),

            active_tool: WorkstationTool::Pointer,
            brush_radius: 24.0,
            brush_falloff: 0.5,
            mask: MaskBuffer::new(256, 256).ok(),
            mask_blend_enabled: false,
            secondary_algo: "Atkinson".into(),
            quick_mask_active: false,
            selection: SelectionBuffer::new(256, 256).ok(),
            selection_mode: SelectionMode::New,
            wand_tolerance: 0.12,
            lasso_points: Vec::new(),
            drag_start_pixel: None,
            drag_current_pixel: None,
            marching_ants_phase: 0.0,
        }
    }

    fn suggested_filename(&self, ext: &str) -> String {
        let stem = self.source_path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or("dither");
        let algo = self.selected_algo.to_lowercase().replace(' ', "-");
        let pal = self.active_palette.name.to_lowercase().replace(' ', "-");
        format!("{stem}_{algo}_{pal}.{ext}")
    }

    fn recents_file_path() -> PathBuf {
        PathBuf::from(".dithercraft_recents.json")
    }

    fn load_recent_files() -> Vec<PathBuf> {
        let path = Self::recents_file_path();
        if let Ok(data) = std::fs::read_to_string(&path)
            && let Ok(paths) = serde_json::from_str::<Vec<PathBuf>>(&data)
        {
            paths.into_iter().filter(|p| p.exists()).collect()
        } else {
            Vec::new()
        }
    }

    fn save_recent_files(&self) {
        let path = Self::recents_file_path();
        if let Ok(json) = serde_json::to_string_pretty(&self.recent_files) {
            let _ = std::fs::write(path, json);
        }
    }

    fn add_recent_file(&mut self, path: PathBuf) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        if self.recent_files.len() > 10 {
            self.recent_files.truncate(10);
        }
        self.save_recent_files();
    }

    pub fn ensure_buffers_dimension(&mut self, w: u32, h: u32) {
        if self.mask.as_ref().map(|m| (m.width, m.height)) != Some((w, h)) {
            self.mask = MaskBuffer::new(w, h).ok();
        }
        if self.selection.as_ref().map(|s| (s.width, s.height)) != Some((w, h)) {
            self.selection = SelectionBuffer::new(w, h).ok();
        }
    }

    pub fn select_all(&mut self) {
        if let Some(ref img) = self.source_image {
            self.ensure_buffers_dimension(img.width, img.height);
            if let Some(sel) = self.selection.as_mut() {
                sel.select_all();
                self.status_text = "Selected entire canvas".into();
            }
        }
    }

    pub fn deselect(&mut self) {
        if let Some(sel) = self.selection.as_mut() {
            sel.clear();
            self.status_text = "Cleared selection".into();
        }
    }

    pub fn invert_selection(&mut self) {
        if let Some(sel) = self.selection.as_mut() {
            sel.invert();
            self.status_text = "Inverted selection".into();
        }
    }

    pub fn convert_selection_to_mask(&mut self) {
        if let Some(sel) = self.selection.as_ref() {
            self.mask = Some(sel.to_mask());
            self.status_text = "Converted selection to mask".into();
        }
    }

    pub fn convert_mask_to_selection(&mut self) {
        if let Some(mask) = self.mask.as_ref() {
            self.selection = Some(SelectionBuffer::from_mask(mask.clone()));
            self.status_text = "Extracted selection from mask".into();
        }
    }

    fn shuffle_adjustments(&mut self) {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let mut state = seed as u64;
        let mut next_f32 = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 33) as f32) / ((1u32 << 31) as f32)
        };
        self.crt_scanline = (next_f32() * 0.7).clamp(0.0, 1.0);
        self.crt_barrel = (next_f32() * 0.25).clamp(0.0, 1.0);
        self.crt_vignette = (next_f32() * 0.6).clamp(0.0, 1.0);
        self.chromatic_shift = (next_f32() * 16.0 - 8.0) as i32;
        self.glitch_intensity = (next_f32() * 0.75).clamp(0.0, 1.0);
        self.glow_intensity = (next_f32() * 1.5).clamp(0.1, 2.0);
        self.glow_radius = (next_f32() * 8.0) as usize + 1;
        self.tonal_cycle_step = (next_f32() * 6.0) as usize;
        self.status_text = "🎲 Shuffled effect adjustments".into();
        self.trigger_render();
    }

    fn transform_rotate_90_cw(&mut self) {
        if let Some(src) = &self.source_image
            && let Ok(rotated) = src.rotate_90_cw()
        {
            self.record_undo_state();
            self.source_image = Some(Arc::new(rotated));
            if !self.source_frames.is_empty() {
                self.source_frames = self.source_frames.iter().filter_map(|f| f.rotate_90_cw().ok().map(Arc::new)).collect();
            }
            self.status_text = "Rotated canvas 90° clockwise".into();
            self.trigger_render();
        }
    }

    fn transform_rotate_180(&mut self) {
        if let Some(src) = &self.source_image
            && let Ok(rotated) = src.rotate_180()
        {
            self.record_undo_state();
            self.source_image = Some(Arc::new(rotated));
            if !self.source_frames.is_empty() {
                self.source_frames = self.source_frames.iter().filter_map(|f| f.rotate_180().ok().map(Arc::new)).collect();
            }
            self.status_text = "Rotated canvas 180°".into();
            self.trigger_render();
        }
    }

    fn transform_rotate_270_cw(&mut self) {
        if let Some(src) = &self.source_image
            && let Ok(rotated) = src.rotate_270_cw()
        {
            self.record_undo_state();
            self.source_image = Some(Arc::new(rotated));
            if !self.source_frames.is_empty() {
                self.source_frames = self.source_frames.iter().filter_map(|f| f.rotate_270_cw().ok().map(Arc::new)).collect();
            }
            self.status_text = "Rotated canvas 90° counter-clockwise".into();
            self.trigger_render();
        }
    }

    fn transform_flip_horizontal(&mut self) {
        if let Some(src) = &self.source_image
            && let Ok(flipped) = src.flip_horizontal()
        {
            self.record_undo_state();
            self.source_image = Some(Arc::new(flipped));
            if !self.source_frames.is_empty() {
                self.source_frames = self.source_frames.iter().filter_map(|f| f.flip_horizontal().ok().map(Arc::new)).collect();
            }
            self.status_text = "Flipped canvas horizontally".into();
            self.trigger_render();
        }
    }

    fn transform_flip_vertical(&mut self) {
        if let Some(src) = &self.source_image
            && let Ok(flipped) = src.flip_vertical()
        {
            self.record_undo_state();
            self.source_image = Some(Arc::new(flipped));
            if !self.source_frames.is_empty() {
                self.source_frames = self.source_frames.iter().filter_map(|f| f.flip_vertical().ok().map(Arc::new)).collect();
            }
            self.status_text = "Flipped canvas vertically".into();
            self.trigger_render();
        }
    }

    fn transform_scale_integer(&mut self, factor: u32) {
        if let Some(src) = &self.source_image
            && let Ok(scaled) = src.scale_integer(factor)
        {
            self.record_undo_state();
            self.source_image = Some(Arc::new(scaled));
            if !self.source_frames.is_empty() {
                self.source_frames = self.source_frames.iter().filter_map(|f| f.scale_integer(factor).ok().map(Arc::new)).collect();
            }
            self.status_text = format!("Scaled canvas {factor}x (nearest neighbor)");
            self.trigger_render();
        }
    }

    fn transform_crop(&mut self, x: u32, y: u32, w: u32, h: u32) {
        if let Some(src) = &self.source_image
            && let Ok(cropped) = src.crop(x, y, w, h)
        {
            self.record_undo_state();
            self.source_image = Some(Arc::new(cropped));
            if !self.source_frames.is_empty() {
                self.source_frames = self.source_frames.iter().filter_map(|f| f.crop(x, y, w, h).ok().map(Arc::new)).collect();
            }
            self.status_text = format!("Cropped canvas to {w}x{h}");
            self.trigger_render();
        }
    }

    fn load_media_file(&mut self, path: &Path) {
        self.source_path = Some(path.to_path_buf());
        self.add_recent_file(path.to_path_buf());

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let is_gif = ext.eq_ignore_ascii_case("gif");
        let is_mp4 = ext.eq_ignore_ascii_case("mp4") || ext.eq_ignore_ascii_case("m4v");
        let is_webm = ext.eq_ignore_ascii_case("webm") || ext.eq_ignore_ascii_case("mkv");

        if is_mp4 && let Ok(bytes) = std::fs::read(path) {
            if let Ok(demuxer) = Mp4Demuxer::parse(&bytes)
                && let Some(track) = demuxer.primary_video_track()
            {
                let mut frames = Vec::new();
                for sample in &track.samples {
                    if let Some(s_bytes) = demuxer.get_sample_bytes(sample, &bytes)
                        && let Ok(dyn_img) = image::load_from_memory(s_bytes)
                    {
                        let rgb = dyn_img.to_rgb8();
                        let (w, h) = rgb.dimensions();
                        if let Ok(buf) = ImageBuffer::from_raw(w, h, rgb.into_raw()) {
                            frames.push(Arc::new(buf));
                            if frames.len() >= 300 {
                                break;
                            }
                        }
                    }
                }
                if !frames.is_empty() {
                    self.timeline_max_frames = frames.len() as u32;
                    self.timeline_frame = 0;
                    self.source_image = Some(frames[0].clone());
                    self.source_frames = frames;
                    let delay = (1000.0 / track.nominal_fps().max(1.0)) as u32;
                    self.frame_delays = vec![delay; self.timeline_max_frames as usize];
                    self.canvas_pan = egui::Vec2::ZERO;
                    self.canvas_zoom = 1.0;
                    self.status_text = format!("Loaded MP4 video ({} frames, {:.1} FPS)", self.source_frames.len(), track.nominal_fps());
                    self.trigger_render();
                } else {
                    self.status_text = format!(
                        "Parsed MP4 track: {}x{}, codec: {}, {} samples",
                        track.width,
                        track.height,
                        track.codec.as_str(),
                        track.samples.len()
                    );
                }
            }
        } else if is_webm && let Ok(bytes) = std::fs::read(path) {
            if let Ok(demuxer) = WebmDemuxer::parse(&bytes)
                && let Some(track) = demuxer.primary_video_track()
            {
                let mut frames = Vec::new();
                for sample in &track.samples {
                    if let Some(s_bytes) = demuxer.get_sample_bytes(sample, &bytes)
                        && let Ok(dyn_img) = image::load_from_memory(s_bytes)
                    {
                        let rgb = dyn_img.to_rgb8();
                        let (w, h) = rgb.dimensions();
                        if let Ok(buf) = ImageBuffer::from_raw(w, h, rgb.into_raw()) {
                            frames.push(Arc::new(buf));
                            if frames.len() >= 300 {
                                break;
                            }
                        }
                    }
                }
                if !frames.is_empty() {
                    self.timeline_max_frames = frames.len() as u32;
                    self.timeline_frame = 0;
                    self.source_image = Some(frames[0].clone());
                    self.source_frames = frames;
                    let delay = (1000.0 / track.nominal_fps().max(1.0)) as u32;
                    self.frame_delays = vec![delay; self.timeline_max_frames as usize];
                    self.canvas_pan = egui::Vec2::ZERO;
                    self.canvas_zoom = 1.0;
                    self.status_text = format!("Loaded WebM video ({} frames, {:.1} FPS)", self.source_frames.len(), track.nominal_fps());
                    self.trigger_render();
                } else {
                    self.status_text =
                        format!("Parsed WebM track: {}x{}, codec: {}, {} blocks", track.width, track.height, track.codec_id, track.samples.len());
                }
            }
        } else if is_gif
            && let Ok(file) = std::fs::File::open(path)
            && let Ok(decoder) = GifDecoder::new(std::io::BufReader::new(file))
            && let Ok(mut raw_frames) = decoder.into_frames().collect_frames()
            && !raw_frames.is_empty()
        {
            if raw_frames.len() > 300 {
                raw_frames.truncate(300);
            }
            let mut frames = Vec::new();
            let mut delays = Vec::new();
            for frame in raw_frames {
                let (w, h) = (frame.buffer().width(), frame.buffer().height());
                if (w as u64) * (h as u64) > 16 * 1024 * 1024 {
                    continue;
                }
                let (num, denom) = frame.delay().numer_denom_ms();
                let delay_ms = num.checked_div(denom).map(|d| d.max(10)).unwrap_or(100);
                let mut rgb_data = Vec::with_capacity((w * h * 3) as usize);
                for px in frame.buffer().pixels() {
                    rgb_data.push(px[0]);
                    rgb_data.push(px[1]);
                    rgb_data.push(px[2]);
                }
                if let Ok(buf) = ImageBuffer::from_raw(w, h, rgb_data) {
                    frames.push(Arc::new(buf));
                    delays.push(delay_ms);
                }
            }
            if !frames.is_empty() {
                self.timeline_max_frames = frames.len() as u32;
                self.timeline_frame = 0;
                self.source_image = Some(frames[0].clone());
                self.source_frames = frames;
                self.frame_delays = delays;
                self.canvas_pan = egui::Vec2::ZERO;
                self.canvas_zoom = 1.0;
                self.status_text = format!("Loaded animated GIF ({} frames)", self.source_frames.len());
                self.trigger_render();
            }
        } else if ext.eq_ignore_ascii_case("svg")
            && let Ok(data) = std::fs::read_to_string(path)
        {
            self.status_text = format!("Loaded SVG vector file ({} bytes)", data.len());
        } else if let Ok(reader) = image::ImageReader::open(path)
            && let Ok(reader) = reader.with_guessed_format()
            && let Ok(dyn_img) = reader.decode()
        {
            let rgb = dyn_img.to_rgb8();
            let (w, h) = rgb.dimensions();
            if let Ok(buf) = ImageBuffer::from_raw(w, h, rgb.into_raw()) {
                let arc_buf = Arc::new(buf);
                self.source_image = Some(arc_buf.clone());
                self.source_frames = vec![arc_buf];
                self.frame_delays = vec![100];
                self.timeline_max_frames = 60;
                self.timeline_frame = 0;
                self.canvas_pan = egui::Vec2::ZERO;
                self.canvas_zoom = 1.0;
                self.status_text = format!("Loaded static image ({w}x{h})");
                self.ensure_buffers_dimension(w, h);
                self.trigger_render();
            }
        }
    }

    pub fn capture_preset(&self, name: impl Into<String>) -> StudioPreset {
        StudioPreset {
            name: name.into(),
            algo: self.selected_algo.clone(),
            palette_name: self.selected_palette_name.clone(),
            metric: self.active_palette.metric,
            tonal_cycle_step: self.tonal_cycle_step,

            glow_enabled: self.glow_enabled,
            glow_threshold: self.glow_threshold,
            glow_radius: self.glow_radius,
            glow_intensity: self.glow_intensity,

            chromatic_enabled: self.chromatic_enabled,
            chromatic_shift: self.chromatic_shift,
            chromatic_radial: self.chromatic_radial,

            glitch_enabled: self.glitch_enabled,
            glitch_mode: self.glitch_mode,
            glitch_intensity: self.glitch_intensity,

            crt_enabled: self.crt_enabled,
            crt_scanline: self.crt_scanline,
            crt_barrel: self.crt_barrel,
            crt_vignette: self.crt_vignette,

            blur_enabled: self.blur_enabled,
            blur_sigma: self.blur_sigma,

            sharpen_enabled: self.sharpen_enabled,
            sharpen_amount: self.sharpen_amount,

            levels_black: self.levels_black,
            levels_white: self.levels_white,
            levels_gamma: self.levels_gamma,
            cvd_mode: self.cvd_mode,

            temporal_enabled: self.temporal_noise_enabled,
            temporal_pattern: self.temporal_pattern,
            temporal_intensity: self.temporal_intensity,

            lut_enabled: self.lut_enabled,
            lut_preset: self.lut_preset_name.clone(),
        }
    }

    pub fn apply_preset(&mut self, preset: &StudioPreset) {
        self.selected_algo = preset.algo.clone();
        self.selected_palette_name = preset.palette_name.clone();
        self.tonal_cycle_step = preset.tonal_cycle_step;

        let mut pal = match preset.palette_name.as_str() {
            "Monochrome" => Palette::monochrome(),
            "Macintosh 1-bit" => Palette::mac_1bit(),
            "CGA Mode 1" => Palette::cga_mode_1(),
            "Cyberpunk" => Palette::cyberpunk(),
            "Apple II" => Palette::apple_ii(),
            "Commodore 64" => Palette::commodore_64(),
            "NES" => Palette::nes(),
            "PICO-8" => Palette::pico_8(),
            _ => Palette::game_boy(),
        };
        pal.metric = preset.metric;
        self.active_palette = pal;

        self.glow_enabled = preset.glow_enabled;
        self.glow_threshold = preset.glow_threshold;
        self.glow_radius = preset.glow_radius;
        self.glow_intensity = preset.glow_intensity;

        self.chromatic_enabled = preset.chromatic_enabled;
        self.chromatic_shift = preset.chromatic_shift;
        self.chromatic_radial = preset.chromatic_radial;

        self.glitch_enabled = preset.glitch_enabled;
        self.glitch_mode = preset.glitch_mode;
        self.glitch_intensity = preset.glitch_intensity;

        self.crt_enabled = preset.crt_enabled;
        self.crt_scanline = preset.crt_scanline;
        self.crt_barrel = preset.crt_barrel;
        self.crt_vignette = preset.crt_vignette;

        self.blur_enabled = preset.blur_enabled;
        self.blur_sigma = preset.blur_sigma;

        self.sharpen_enabled = preset.sharpen_enabled;
        self.sharpen_amount = preset.sharpen_amount;

        self.temporal_noise_enabled = preset.temporal_enabled;
        self.temporal_pattern = preset.temporal_pattern;
        self.temporal_intensity = preset.temporal_intensity;

        self.levels_black = preset.levels_black;
        self.levels_white = preset.levels_white;
        self.levels_gamma = preset.levels_gamma;
        self.cvd_mode = preset.cvd_mode;

        self.lut_enabled = preset.lut_enabled;
        self.lut_preset_name = preset.lut_preset.clone();
        self.custom_lut = None;

        self.trigger_render();
    }

    pub fn record_undo_state(&mut self) {
        let snapshot = self.capture_preset("Snapshot");
        if self.undo_stack.last() != Some(&snapshot) {
            if self.undo_stack.len() >= 30 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(snapshot);
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            let current = self.capture_preset("Current");
            self.redo_stack.push(current);
            self.apply_preset(&prev);
            self.status_text = format!("Undo: restored {}", prev.algo);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = self.capture_preset("Current");
            self.undo_stack.push(current);
            self.apply_preset(&next);
            self.status_text = format!("Redo: restored {}", next.algo);
        }
    }

    fn prepare_render_frame(&self, frame_idx: usize) -> Arc<ImageBuffer> {
        let mapped_idx = if self.source_frames.len() > 1 {
            self.cadence_mode.map_frame_index(frame_idx, self.target_fps).min(self.source_frames.len() - 1)
        } else {
            0
        };
        let mut base_img = if self.onion_skin_enabled && self.source_frames.len() > 1 {
            render_onion_skin(&self.source_frames, mapped_idx, &self.onion_skin_config).unwrap_or_else(|_| (*self.source_frames[mapped_idx]).clone())
        } else if !self.source_frames.is_empty() {
            (*self.source_frames[mapped_idx]).clone()
        } else if let Some(ref src) = self.source_image {
            (**src).clone()
        } else {
            ImageBuffer { width: 1, height: 1, data: vec![0, 0, 0] }
        };
        if self.cadence_mode == CadenceMode::InterlacedFields {
            base_img = self.cadence_mode.apply_interlacing(&base_img, frame_idx);
        }
        Arc::new(base_img)
    }

    fn set_current_frame(&mut self, frame: u32) {
        self.timeline_frame = frame;
        self.frame_counter = frame;
        if !self.source_frames.is_empty() || self.source_image.is_some() {
            self.source_image = Some(self.prepare_render_frame(frame as usize));
        }
        self.trigger_render();
    }

    fn trigger_render(&mut self) {
        self.trigger_render_mode(false);
    }

    fn trigger_render_preview(&mut self) {
        self.trigger_render_mode(true);
    }

    fn trigger_render_mode(&mut self, is_preview: bool) {
        let Some(mut source) = self.source_image.clone() else { return };
        if is_preview
            && (source.width > 512 || source.height > 512)
            && let Ok(down) = source.downsample(512, 512)
        {
            source = Arc::new(down);
        }
        let tx = self.tx.clone();
        let algo_name = self.selected_algo.clone();

        let mut tonal_step = self.tonal_cycle_step;
        let mut crt_scanline = self.crt_scanline;
        let mut glitch_intensity = self.glitch_intensity;
        let mut chromatic_shift = self.chromatic_shift;

        if self.audio_reactive_enabled
            && let Some(ref track) = self.audio_track
        {
            let spec = track.get_frame_spectrum(self.timeline_frame, self.target_fps);
            self.audio_spectrum = spec;
            if spec.bass > 0.35 {
                tonal_step = (tonal_step + (spec.bass * 4.0) as usize) % 9;
            }
            glitch_intensity = (glitch_intensity + spec.bass * 0.4).min(1.0);
            crt_scanline = (crt_scanline + spec.mid * 0.3).min(1.0);
            chromatic_shift += (spec.treble * 8.0) as i32;
        }

        let palette = self.active_palette.cycle_tonal(tonal_step);

        let mut effect_stack = self.effect_stack.clone();
        for node in &mut effect_stack.nodes {
            match &mut node.kind {
                EffectKind::Glow(glow) => {
                    node.enabled = self.glow_enabled;
                    glow.threshold = self.glow_threshold;
                    glow.radius = self.glow_radius;
                    glow.intensity = self.glow_intensity;
                }
                EffectKind::Chromatic(chroma) => {
                    node.enabled = self.chromatic_enabled;
                    chroma.shift_x = chromatic_shift;
                    chroma.radial = self.chromatic_radial;
                }
                EffectKind::Glitch(glitch) => {
                    node.enabled = self.glitch_enabled;
                    glitch.mode = self.glitch_mode;
                    glitch.intensity = glitch_intensity;
                }
                EffectKind::Crt(crt) => {
                    node.enabled = self.crt_enabled;
                    crt.scanline_intensity = crt_scanline;
                    crt.barrel_curvature = self.crt_barrel;
                    crt.vignette_intensity = self.crt_vignette;
                }
                EffectKind::Blur(blur) => {
                    node.enabled = self.blur_enabled;
                    blur.sigma = self.blur_sigma;
                }
                EffectKind::Sharpen(sharpen) => {
                    node.enabled = self.sharpen_enabled;
                    sharpen.amount = self.sharpen_amount;
                }
                _ => {}
            }
        }

        let temporal_opt =
            if self.temporal_noise_enabled { Some((self.temporal_pattern, self.frame_counter, self.temporal_intensity)) } else { None };

        let lut_opt = if self.lut_enabled {
            if let Some(ref custom) = self.custom_lut {
                Some(custom.clone())
            } else {
                Some(match self.lut_preset_name.as_str() {
                    "Vintage Warm 1970" => Cube3dLut::vintage_warm_1970(32),
                    "Amber Phosphor" => Cube3dLut::amber_phosphor(32),
                    "Cyber Matrix" => Cube3dLut::cyber_matrix(32),
                    "Bleach Bypass" => Cube3dLut::bleach_bypass(32),
                    _ => Cube3dLut::technicolor_2strip(32),
                })
            }
        } else {
            None
        };

        let temporal_coherence = if self.temporal_coherence_enabled {
            Some((
                self.temporal_coherence_damping,
                self.temporal_coherence_threshold,
                self.last_rendered_source.clone(),
                self.last_rendered_output.clone(),
            ))
        } else {
            None
        };

        let levels_black = self.levels_black;
        let levels_white = self.levels_white;
        let levels_gamma = self.levels_gamma;
        let cvd_mode = self.cvd_mode;

        let cmyk_dot_shape = self.cmyk_dot_shape;
        let cmyk_dot_gain = self.cmyk_dot_gain;
        let tonal_shadow_algo = self.tonal_shadow_algo.clone();
        let tonal_mid_algo = self.tonal_mid_algo.clone();
        let tonal_highlight_algo = self.tonal_highlight_algo.clone();
        let tonal_shadow_threshold = self.tonal_shadow_threshold;
        let tonal_highlight_threshold = self.tonal_highlight_threshold;
        let tonal_feather = self.tonal_feather;
        let custom_matrix = self.custom_matrix.clone();
        let rd_morphology = self.rd_morphology;
        let rd_steps = self.rd_steps;
        let flow_scale = self.flow_scale;
        let flow_swirl = self.flow_swirl;

        let mask_blend_opt = if self.mask_blend_enabled { self.mask.clone().map(|m| (m, self.secondary_algo.clone())) } else { None };

        self.is_rendering = true;
        let gen_id = self.render_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let cancel_token = self.render_generation.clone();
        let ctx = self.egui_ctx.clone();

        rayon::spawn(move || {
            let start = Instant::now();
            if cancel_token.load(Ordering::Relaxed) != gen_id {
                return;
            }

            let t_dither = Instant::now();
            let algo: Box<dyn DitherAlgorithm> = if algo_name.to_lowercase().contains("cmyk") {
                Box::new(CmykHalftone { dot_shape: cmyk_dot_shape, dot_gain: cmyk_dot_gain, ..Default::default() })
            } else if algo_name.to_lowercase().contains("tonal band") {
                Box::new(TonalBandDither::new(
                    tonal_shadow_threshold,
                    tonal_highlight_threshold,
                    tonal_feather,
                    create_algorithm(&tonal_shadow_algo).unwrap_or_else(|| Box::new(Bayer8x8)),
                    create_algorithm(&tonal_mid_algo).unwrap_or_else(|| Box::new(FloydSteinberg::default())),
                    create_algorithm(&tonal_highlight_algo).unwrap_or_else(|| Box::new(Atkinson::default())),
                ))
            } else if algo_name.to_lowercase().contains("custom threshold matrix") || algo_name.to_lowercase() == "custom" {
                Box::new(custom_matrix)
            } else if algo_name.to_lowercase().contains("morphogenesis") || algo_name.to_lowercase().contains("reaction-diffusion") {
                Box::new(ReactionDiffusionDither::new(ReactionDiffusionConfig { morphology: rd_morphology, steps: rd_steps, ..Default::default() }))
            } else if algo_name.to_lowercase().contains("flow field") || algo_name.to_lowercase().contains("vector flow") {
                Box::new(FlowFieldSteeredDither::new(FlowFieldConfig { scale: flow_scale, swirl: flow_swirl, angle_offset: 0.0 }))
            } else {
                create_algorithm(&algo_name).unwrap_or_else(|| Box::new(FloydSteinberg::default()))
            };

            let mut input_buf = (*source).clone();
            if let Some(ref lut) = lut_opt {
                lut.apply_buffer(&mut input_buf);
            }
            if levels_black > 0 || levels_white < 255 || (levels_gamma - 1.0).abs() > 0.001 {
                input_buf.adjust_levels(levels_black, levels_white, levels_gamma);
            }

            let sec_algo: Option<Box<dyn DitherAlgorithm>> = if let Some((_, ref sec_name)) = mask_blend_opt {
                create_algorithm(sec_name).or_else(|| Some(Box::new(Atkinson::default())))
            } else {
                None
            };

            let mut out = if let (Some((mask, _)), Some(sec_inst)) = (&mask_blend_opt, &sec_algo) {
                let out_a = algo.apply(&input_buf, &palette).unwrap_or_else(|_| input_buf.clone());
                let out_b = sec_inst.apply(&input_buf, &palette).unwrap_or_else(|_| input_buf.clone());
                mask.blend_images(&out_a, &out_b).unwrap_or(out_a)
            } else {
                algo.apply(&input_buf, &palette).unwrap_or_else(|_| input_buf.clone())
            };
            let dither_ms = t_dither.elapsed().as_secs_f64() * 1000.0;

            if cancel_token.load(Ordering::Relaxed) != gen_id {
                return;
            }

            let t_fx = Instant::now();
            // Downstream Effect Stack
            out = effect_stack.apply(&out).unwrap_or(out);

            if let Some((pattern, frame, intensity)) = temporal_opt {
                out = pattern.apply(&out, frame, intensity).unwrap_or(out);
            }
            if let Some((damp, thresh, Some(ref prev_s), Some(ref prev_o))) = temporal_coherence {
                let filter = TemporalCoherenceFilter::new(TemporalCoherenceConfig { damping: damp, motion_threshold: thresh });
                out = filter.apply(prev_s, &input_buf, prev_o, &out).unwrap_or(out);
            }
            if cvd_mode != CvdMode::Normal {
                out.apply_cvd_simulation(cvd_mode);
            }
            let effects_ms = t_fx.elapsed().as_secs_f64() * 1000.0;

            if cancel_token.load(Ordering::Relaxed) != gen_id {
                return;
            }

            let t_hist = Instant::now();
            // Compute swatch utilization counts for live histogram
            let mut swatch_counts = vec![0usize; palette.swatches.len()];
            if !palette.swatches.is_empty() {
                for chunk in out.data.chunks_exact(3) {
                    let px = RgbColor::new(chunk[0], chunk[1], chunk[2]);
                    let idx = palette.find_nearest_index(&px);
                    if idx < swatch_counts.len() {
                        swatch_counts[idx] += 1;
                    }
                }
            }
            let histogram_ms = t_hist.elapsed().as_secs_f64() * 1000.0;

            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            let _ = tx.send(RenderJobResult { buffer: out, duration_ms, dither_ms, effects_ms, histogram_ms, gen_id, swatch_counts });
            ctx.request_repaint();
        });
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if let Some(res) = self.rendered_image.as_ref() {
            let width = res.width as usize;
            let height = res.height as usize;
            let mut color_data = Vec::with_capacity(width * height * 4);

            if self.split_view_enabled
                && let Some(src) = self.source_image.as_ref()
                && src.width == res.width
                && src.height == res.height
            {
                let split_x = (res.width as f32 * self.split_position).clamp(0.0, res.width as f32) as usize;
                let split_y = (res.height as f32 * self.split_position).clamp(0.0, res.height as f32) as usize;
                let tile = self.split_checker_tile_size.max(4);

                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) * 3;
                        match self.split_mode {
                            SplitViewMode::Vertical => {
                                if x == split_x {
                                    color_data.extend_from_slice(&[255, 255, 255, 255]);
                                } else if x < split_x {
                                    color_data.extend_from_slice(&[src.data[idx], src.data[idx + 1], src.data[idx + 2], 255]);
                                } else {
                                    color_data.extend_from_slice(&[res.data[idx], res.data[idx + 1], res.data[idx + 2], 255]);
                                }
                            }
                            SplitViewMode::Horizontal => {
                                if y == split_y {
                                    color_data.extend_from_slice(&[255, 255, 255, 255]);
                                } else if y < split_y {
                                    color_data.extend_from_slice(&[src.data[idx], src.data[idx + 1], src.data[idx + 2], 255]);
                                } else {
                                    color_data.extend_from_slice(&[res.data[idx], res.data[idx + 1], res.data[idx + 2], 255]);
                                }
                            }
                            SplitViewMode::Checkerboard => {
                                let check = ((x / tile) + (y / tile)) & 1 == 0;
                                if check {
                                    color_data.extend_from_slice(&[src.data[idx], src.data[idx + 1], src.data[idx + 2], 255]);
                                } else {
                                    color_data.extend_from_slice(&[res.data[idx], res.data[idx + 1], res.data[idx + 2], 255]);
                                }
                            }
                            SplitViewMode::Difference => {
                                let dr = (src.data[idx] as i32 - res.data[idx] as i32).abs();
                                let dg = (src.data[idx + 1] as i32 - res.data[idx + 1] as i32).abs();
                                let db = (src.data[idx + 2] as i32 - res.data[idx + 2] as i32).abs();
                                let max_d = (dr.max(dg).max(db) as f32 / 255.0).clamp(0.0, 1.0);
                                let (hr, hg, hb) = if max_d < 0.25 {
                                    let t = max_d / 0.25;
                                    (0, (t * 128.0) as u8, (t * 255.0) as u8)
                                } else if max_d < 0.5 {
                                    let t = (max_d - 0.25) / 0.25;
                                    (0, (128.0 + t * 127.0) as u8, 255)
                                } else if max_d < 0.75 {
                                    let t = (max_d - 0.5) / 0.25;
                                    ((t * 255.0) as u8, 255, (255.0 * (1.0 - t)) as u8)
                                } else {
                                    let t = (max_d - 0.75) / 0.25;
                                    (255, (255.0 * (1.0 - t * 0.5)) as u8, (t * 255.0) as u8)
                                };
                                color_data.extend_from_slice(&[hr, hg, hb, 255]);
                            }
                        }
                    }
                }
            } else {
                for chunk in res.data.chunks_exact(3) {
                    color_data.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
            }

            if self.quick_mask_active
                && let Some(ref mask) = self.mask
                && mask.width as usize == width
                && mask.height as usize == height
            {
                for (pixel_rgba, &m_val) in color_data.chunks_exact_mut(4).zip(&mask.data) {
                    if m_val > 0 {
                        let alpha = (m_val as f32 / 255.0) * 0.45;
                        let inv = 1.0 - alpha;
                        pixel_rgba[0] = ((pixel_rgba[0] as f32 * inv) + (255.0 * alpha)).round() as u8;
                        pixel_rgba[1] = ((pixel_rgba[1] as f32 * inv) + (30.0 * alpha)).round() as u8;
                        pixel_rgba[2] = ((pixel_rgba[2] as f32 * inv) + (60.0 * alpha)).round() as u8;
                    }
                }
            }

            let color_image = ColorImage::from_rgba_unmultiplied([width, height], &color_data);
            if let Some(tex) = self.texture.as_mut() {
                tex.set(color_image, TextureOptions::NEAREST);
            } else {
                self.texture = Some(ctx.load_texture("rendered_preview", color_image, TextureOptions::NEAREST));
            }
        }
    }
}

#[inline]
fn check_slider(ui: &mut egui::Ui, s: egui::Slider, changed: &mut bool, preview: &mut bool) {
    let r = ui.add(s);
    if r.dragged() {
        *preview = true;
    } else if r.drag_stopped() || r.changed() {
        *changed = true;
    }
}

impl eframe::App for DitherCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll worker thread results
        while let Ok(result) = self.rx.try_recv() {
            if result.gen_id == self.render_generation.load(Ordering::Relaxed) {
                let total_px = (result.buffer.width * result.buffer.height) as f32;
                if total_px > 0.0 {
                    self.swatch_percentages = result.swatch_counts.iter().map(|&cnt| (cnt as f32 / total_px) * 100.0).collect();
                }
                if let Some(src) = &self.source_image {
                    self.last_rendered_source = Some((**src).clone());
                }
                self.last_rendered_output = Some(result.buffer.clone());
                self.rendered_image = Some(result.buffer);
                self.render_latency_ms = result.duration_ms;
                self.telemetry_dither_ms = result.dither_ms;
                self.telemetry_effects_ms = result.effects_ms;
                self.telemetry_histogram_ms = result.histogram_ms;
                self.is_rendering = false;
                self.update_texture(ctx);
            }
        }

        // Apply theme
        self.theme.apply(ctx);

        // Handle Undo / Redo keyboard shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            if ctx.input(|i| i.modifiers.shift) {
                self.redo();
            } else {
                self.undo();
            }
        } else if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)) {
            self.redo();
        }

        // Handle Instant A/B Preset Snapshots (F5-F8 recall, Shift+F5-F8 save)
        for (idx, key) in [(0, egui::Key::F5), (1, egui::Key::F6), (2, egui::Key::F7), (3, egui::Key::F8)] {
            if ctx.input(|i| i.key_pressed(key)) {
                let slot_char = ['A', 'B', 'C', 'D'][idx];
                if ctx.input(|i| i.modifiers.shift) {
                    let snap = self.capture_preset(format!("Snapshot {slot_char}"));
                    self.status_text = format!("Stored Snapshot {slot_char}: {} ({})", snap.algo, snap.palette_name);
                    self.snapshots[idx] = Some(snap);
                } else if let Some(snap) = self.snapshots[idx].clone() {
                    self.record_undo_state();
                    self.apply_preset(&snap);
                    self.status_text = format!("Recalled Snapshot {slot_char}: {} ({})", snap.algo, snap.palette_name);
                    self.trigger_render();
                } else {
                    self.status_text = format!("Snapshot {slot_char} is empty. Use Shift+F{} or right-click button to store.", idx + 5);
                }
            }
        }

        // Toggle Precision Magnifier Loupe (L)
        if ctx.input(|i| !i.modifiers.command && !i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(egui::Key::L)) {
            self.show_loupe = !self.show_loupe;
        }

        // Tools & Masking Shortcuts
        if !ctx.input(|i| i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt) {
            if ctx.input(|i| i.key_pressed(egui::Key::V)) {
                self.active_tool = WorkstationTool::Pointer;
                self.status_text = "Tool: Pointer (V)".into();
            } else if ctx.input(|i| i.key_pressed(egui::Key::B)) {
                self.active_tool = WorkstationTool::BrushMask;
                self.status_text = "Tool: Brush Mask (B)".into();
            } else if ctx.input(|i| i.key_pressed(egui::Key::E)) {
                self.active_tool = WorkstationTool::EraserMask;
                self.status_text = "Tool: Eraser Mask (E)".into();
            } else if ctx.input(|i| i.key_pressed(egui::Key::M)) {
                self.active_tool = WorkstationTool::RectMarquee;
                self.status_text = "Tool: Rect Marquee (M)".into();
            } else if ctx.input(|i| i.key_pressed(egui::Key::W)) {
                self.active_tool = WorkstationTool::MagicWand;
                self.status_text = "Tool: Magic Wand (W)".into();
            } else if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
                self.quick_mask_active = !self.quick_mask_active;
                self.status_text = format!("Quick Mask: {}", if self.quick_mask_active { "ON" } else { "OFF" });
                self.update_texture(ctx);
            }
        }

        // Selection Shortcuts (Ctrl+A, Ctrl+D, Ctrl+Shift+I)
        if ctx.input(|i| i.modifiers.command || i.modifiers.ctrl) {
            if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                self.select_all();
            } else if ctx.input(|i| i.key_pressed(egui::Key::D)) {
                self.deselect();
            } else if ctx.input(|i| i.modifiers.shift && i.key_pressed(egui::Key::I)) {
                self.invert_selection();
            }
        }

        // Toggle Keyboard Shortcuts Cheatsheet (F1 or ?)
        if ctx.input(|i| i.key_pressed(egui::Key::F1) || (i.modifiers.shift && i.key_pressed(egui::Key::Slash))) {
            self.show_shortcuts_dialog = !self.show_shortcuts_dialog;
        }

        // Handle Drag & Drop
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            for file in dropped {
                if let Some(path) = file.path {
                    self.load_media_file(&path);
                    break;
                }
            }
        }

        // Top Menu Bar
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Image, GIF, or Video...").clicked() {
                        let path = rfd::FileDialog::new()
                            .add_filter(
                                "Media Files",
                                &[
                                    "png", "jpg", "jpeg", "webp", "bmp", "gif", "mp4", "m4v", "webm", "mkv", "tiff", "tif", "tga", "ico", "qoi",
                                    "hdr", "svg",
                                ],
                            )
                            .pick_file();
                        if let Some(path) = path {
                            self.load_media_file(&path);
                        }
                        ui.close_menu();
                    }

                    ui.menu_button("Open Recent", |ui| {
                        if self.recent_files.is_empty() {
                            ui.label("No recent files");
                        } else {
                            let mut selected = None;
                            for p in &self.recent_files {
                                let label = p.file_name().and_then(|n| n.to_str()).unwrap_or("File");
                                if ui.button(label).on_hover_text(p.display().to_string()).clicked() {
                                    selected = Some(p.clone());
                                    ui.close_menu();
                                }
                            }
                            if let Some(p) = selected {
                                self.load_media_file(&p);
                            }
                        }
                    });

                    if ui.button("Import Audio Track (.wav)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("WAV Audio", &["wav"]).pick_file()
                            && let Ok(bytes) = std::fs::read(&path)
                            && let Ok(track) = AudioTrack::parse_wav(&bytes)
                        {
                            let dur = if track.sample_rate > 0 && track.channels > 0 {
                                (track.samples.len() as f64) / (track.sample_rate as f64 * track.channels as f64)
                            } else {
                                0.0
                            };
                            self.status_text = format!("Loaded WAV audio track: {} Hz, {} ch, {:.2}s", track.sample_rate, track.channels, dur);
                            self.audio_track = Some(track);
                            self.audio_reactive_enabled = true;
                            self.trigger_render();
                        }
                        ui.close_menu();
                    }

                    if ui.button("Save Rendered PNG...").clicked() {
                        let fname = self.suggested_filename("png");
                        if let Some(ref img) = self.rendered_image
                            && let Some(path) = rfd::FileDialog::new().set_file_name(&fname).add_filter("PNG Image", &["png"]).save_file()
                        {
                            let _ = image::save_buffer(&path, &img.data, img.width, img.height, image::ExtendedColorType::Rgb8);
                            self.status_text = format!("Saved PNG to {}", path.display());
                        }
                        ui.close_menu();
                    }

                    if ui.button("Save Rendered JPEG...").clicked() {
                        let fname = self.suggested_filename("jpg");
                        if let Some(ref img) = self.rendered_image
                            && let Some(path) = rfd::FileDialog::new().set_file_name(&fname).add_filter("JPEG Image", &["jpg", "jpeg"]).save_file()
                        {
                            let _ = image::save_buffer(&path, &img.data, img.width, img.height, image::ExtendedColorType::Rgb8);
                            self.status_text = format!("Saved JPEG to {}", path.display());
                        }
                        ui.close_menu();
                    }

                    if ui.button("Save Rendered WebP...").clicked() {
                        let fname = self.suggested_filename("webp");
                        if let Some(ref img) = self.rendered_image
                            && let Some(path) = rfd::FileDialog::new().set_file_name(&fname).add_filter("WebP Image", &["webp"]).save_file()
                        {
                            let _ = image::save_buffer(&path, &img.data, img.width, img.height, image::ExtendedColorType::Rgb8);
                            self.status_text = format!("Saved WebP to {}", path.display());
                        }
                        ui.close_menu();
                    }

                    if let Some(ref track) = self.audio_track
                        && ui.button("Export Synchronized Audio (.wav)...").clicked()
                    {
                        let num_frames = if self.source_frames.len() > 1 { self.source_frames.len() as u32 } else { self.timeline_max_frames };
                        let audio_bytes = track.export_wav_slice(num_frames, self.target_fps);
                        let fname = self.suggested_filename("wav");
                        if let Some(path) = rfd::FileDialog::new().set_file_name(fname).add_filter("WAV Audio", &["wav"]).save_file()
                            && let Ok(()) = std::fs::write(&path, audio_bytes)
                        {
                            self.status_text = format!("Exported synchronized WAV audio to {}", path.display());
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Animated GIF...").clicked() {
                        if let Some(path) =
                            rfd::FileDialog::new().set_file_name(self.suggested_filename("gif")).add_filter("GIF Animation", &["gif"]).save_file()
                        {
                            let num_frames = if self.source_frames.len() > 1 { self.source_frames.len() } else { self.timeline_max_frames as usize };
                            let frames_to_render: Vec<Arc<ImageBuffer>> = (0..num_frames).map(|i| self.prepare_render_frame(i)).collect();

                            if !frames_to_render.is_empty()
                                && let Ok(out_file) = std::fs::File::create(&path)
                            {
                                let mut encoder = GifEncoder::new(std::io::BufWriter::new(out_file));
                                let _ = encoder.set_repeat(Repeat::Infinite);

                                let delays = self.frame_delays.clone();
                                let default_delay_ms = 1000 / self.target_fps.max(1);

                                let algo_name = self.selected_algo.clone();
                                let palette = self.active_palette.clone();
                                let temporal_noise =
                                    if self.temporal_noise_enabled { Some((self.temporal_pattern, self.temporal_intensity)) } else { None };
                                let glow_opt = if self.glow_enabled {
                                    Some(EpsilonGlow { threshold: self.glow_threshold, radius: self.glow_radius, intensity: self.glow_intensity })
                                } else {
                                    None
                                };
                                let crt_opt = if self.crt_enabled {
                                    Some(VintageCrtSimulation {
                                        scanline_intensity: self.crt_scanline,
                                        barrel_curvature: self.crt_barrel,
                                        vignette_intensity: self.crt_vignette,
                                    })
                                } else {
                                    None
                                };
                                let blur_opt = if self.blur_enabled { Some(GaussianBlur::new(self.blur_sigma)) } else { None };
                                let sharpen_opt = if self.sharpen_enabled { Some(Sharpen::new(self.sharpen_amount, 1.0)) } else { None };
                                let chromatic_opt = if self.chromatic_enabled {
                                    Some(ChromaticAberration { shift_x: self.chromatic_shift, shift_y: 0, radial: self.chromatic_radial })
                                } else {
                                    None
                                };
                                let glitch_opt = if self.glitch_enabled {
                                    Some(JpegGlitchCore { mode: self.glitch_mode, intensity: self.glitch_intensity, block_size: 8, seed: 42 })
                                } else {
                                    None
                                };

                                use rayon::prelude::*;
                                let mut rendered: Vec<_> = frames_to_render
                                    .par_iter()
                                    .enumerate()
                                    .map(|(frame_idx, src_buf)| {
                                        let mut current = (**src_buf).clone();
                                        if let Some((pattern, intensity)) = temporal_noise {
                                            current = pattern.apply(&current, frame_idx as u32, intensity).unwrap_or(current);
                                        }
                                        let algo = create_algorithm(&algo_name).unwrap_or_else(|| Box::new(FloydSteinberg::default()));
                                        let mut out = algo.apply(&current, &palette).unwrap_or(current);
                                        if let Some(glow) = &glow_opt {
                                            out = glow.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(chroma) = &chromatic_opt {
                                            out = chroma.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(blur) = &blur_opt {
                                            out = blur.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(sharp) = &sharpen_opt {
                                            out = sharp.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(crt) = &crt_opt {
                                            out = crt.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(glitch) = &glitch_opt {
                                            out = glitch.apply(&out).unwrap_or(out);
                                        }

                                        let delay_ms = delays.get(frame_idx).copied().unwrap_or(default_delay_ms);
                                        (out, delay_ms)
                                    })
                                    .collect();

                                if self.temporal_coherence_enabled {
                                    let raw_sources: Vec<ImageBuffer> = frames_to_render.iter().map(|b| (**b).clone()).collect();
                                    let dithered: Vec<ImageBuffer> = rendered.iter().map(|(b, _)| b.clone()).collect();
                                    let filter = TemporalCoherenceFilter::new(TemporalCoherenceConfig {
                                        damping: self.temporal_coherence_damping,
                                        motion_threshold: self.temporal_coherence_threshold,
                                    });
                                    if let Ok(filtered) = filter.apply_sequence(&raw_sources, &dithered) {
                                        for (i, f) in filtered.into_iter().enumerate() {
                                            if i < rendered.len() {
                                                rendered[i].0 = f;
                                            }
                                        }
                                    }
                                }

                                for (buf, delay_ms) in rendered {
                                    let mut rgba_raw = Vec::with_capacity((buf.width * buf.height * 4) as usize);
                                    for chunk in buf.data.chunks_exact(3) {
                                        rgba_raw.push(chunk[0]);
                                        rgba_raw.push(chunk[1]);
                                        rgba_raw.push(chunk[2]);
                                        rgba_raw.push(255);
                                    }
                                    if let Some(rgba_img) = RgbaImage::from_raw(buf.width, buf.height, rgba_raw) {
                                        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(delay_ms as u64));
                                        let frame = Frame::from_parts(rgba_img, 0, 0, delay);
                                        let _ = encoder.encode_frame(frame);
                                    }
                                }
                                self.status_text = format!("Exported animated GIF ({num_frames} frames)");
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Animated APNG...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(self.suggested_filename("png"))
                            .add_filter("APNG Animation", &["png", "apng"])
                            .save_file()
                        {
                            let num_frames = if self.source_frames.len() > 1 { self.source_frames.len() } else { self.timeline_max_frames as usize };
                            let frames_to_render: Vec<Arc<ImageBuffer>> = (0..num_frames).map(|i| self.prepare_render_frame(i)).collect();

                            if !frames_to_render.is_empty()
                                && let Ok(out_file) = std::fs::File::create(&path)
                            {
                                let first_frame = &frames_to_render[0];
                                let mut writer_buf = std::io::BufWriter::new(out_file);
                                let mut encoder = png::Encoder::new(&mut writer_buf, first_frame.width, first_frame.height);
                                encoder.set_color(png::ColorType::Rgba);
                                encoder.set_depth(png::BitDepth::Eight);
                                let _ = encoder.set_animated(num_frames as u32, 0);
                                let delays = self.frame_delays.clone();
                                let default_delay_ms = 1000 / self.target_fps.max(1);
                                let _ = encoder.set_frame_delay(default_delay_ms as u16, 1000);

                                let algo_name = self.selected_algo.clone();
                                let palette = self.active_palette.clone();
                                let temporal_noise =
                                    if self.temporal_noise_enabled { Some((self.temporal_pattern, self.temporal_intensity)) } else { None };
                                let glow_opt = if self.glow_enabled {
                                    Some(EpsilonGlow { threshold: self.glow_threshold, radius: self.glow_radius, intensity: self.glow_intensity })
                                } else {
                                    None
                                };
                                let crt_opt = if self.crt_enabled {
                                    Some(VintageCrtSimulation {
                                        scanline_intensity: self.crt_scanline,
                                        barrel_curvature: self.crt_barrel,
                                        vignette_intensity: self.crt_vignette,
                                    })
                                } else {
                                    None
                                };
                                let blur_opt = if self.blur_enabled { Some(GaussianBlur::new(self.blur_sigma)) } else { None };
                                let sharpen_opt = if self.sharpen_enabled { Some(Sharpen::new(self.sharpen_amount, 1.0)) } else { None };
                                let chromatic_opt = if self.chromatic_enabled {
                                    Some(ChromaticAberration { shift_x: self.chromatic_shift, shift_y: 0, radial: self.chromatic_radial })
                                } else {
                                    None
                                };
                                let glitch_opt = if self.glitch_enabled {
                                    Some(JpegGlitchCore { mode: self.glitch_mode, intensity: self.glitch_intensity, block_size: 8, seed: 42 })
                                } else {
                                    None
                                };

                                use rayon::prelude::*;
                                let mut rendered: Vec<_> = frames_to_render
                                    .par_iter()
                                    .enumerate()
                                    .map(|(frame_idx, src_buf)| {
                                        let mut current = (**src_buf).clone();
                                        if let Some((pattern, intensity)) = temporal_noise {
                                            current = pattern.apply(&current, frame_idx as u32, intensity).unwrap_or(current);
                                        }
                                        let algo = create_algorithm(&algo_name).unwrap_or_else(|| Box::new(FloydSteinberg::default()));
                                        let mut out = algo.apply(&current, &palette).unwrap_or(current);
                                        if let Some(glow) = &glow_opt {
                                            out = glow.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(chroma) = &chromatic_opt {
                                            out = chroma.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(blur) = &blur_opt {
                                            out = blur.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(sharp) = &sharpen_opt {
                                            out = sharp.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(crt) = &crt_opt {
                                            out = crt.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(glitch) = &glitch_opt {
                                            out = glitch.apply(&out).unwrap_or(out);
                                        }

                                        let delay_ms = delays.get(frame_idx).copied().unwrap_or(default_delay_ms);
                                        (out, delay_ms)
                                    })
                                    .collect();

                                if self.temporal_coherence_enabled {
                                    let raw_sources: Vec<ImageBuffer> = frames_to_render.iter().map(|b| (**b).clone()).collect();
                                    let dithered: Vec<ImageBuffer> = rendered.iter().map(|(b, _)| b.clone()).collect();
                                    let filter = TemporalCoherenceFilter::new(TemporalCoherenceConfig {
                                        damping: self.temporal_coherence_damping,
                                        motion_threshold: self.temporal_coherence_threshold,
                                    });
                                    if let Ok(filtered) = filter.apply_sequence(&raw_sources, &dithered) {
                                        for (i, f) in filtered.into_iter().enumerate() {
                                            if i < rendered.len() {
                                                rendered[i].0 = f;
                                            }
                                        }
                                    }
                                }

                                if let Ok(mut png_writer) = encoder.write_header() {
                                    for (buf, delay_ms) in rendered {
                                        let mut rgba_raw = Vec::with_capacity((buf.width * buf.height * 4) as usize);
                                        for chunk in buf.data.chunks_exact(3) {
                                            rgba_raw.push(chunk[0]);
                                            rgba_raw.push(chunk[1]);
                                            rgba_raw.push(chunk[2]);
                                            rgba_raw.push(255);
                                        }
                                        let _ = png_writer.set_frame_delay(delay_ms as u16, 1000);
                                        let _ = png_writer.write_image_data(&rgba_raw);
                                    }
                                    self.status_text = format!("Exported animated APNG ({num_frames} frames)");
                                }
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Animated WebP...").clicked() {
                        if let Some(path) =
                            rfd::FileDialog::new().set_file_name(self.suggested_filename("webp")).add_filter("WebP Animation", &["webp"]).save_file()
                        {
                            let num_frames = if self.source_frames.len() > 1 { self.source_frames.len() } else { self.timeline_max_frames as usize };
                            let frames_to_render: Vec<Arc<ImageBuffer>> = (0..num_frames).map(|i| self.prepare_render_frame(i)).collect();

                            if !frames_to_render.is_empty() {
                                let first_frame = &frames_to_render[0];
                                let mut muxer = WebpAnimationMuxer::new(first_frame.width, first_frame.height);
                                let delays = self.frame_delays.clone();
                                let default_delay_ms = 1000 / self.target_fps.max(1);

                                let algo_name = self.selected_algo.clone();
                                let palette = self.active_palette.clone();
                                let temporal_noise =
                                    if self.temporal_noise_enabled { Some((self.temporal_pattern, self.temporal_intensity)) } else { None };
                                let glow_opt = if self.glow_enabled {
                                    Some(EpsilonGlow { threshold: self.glow_threshold, radius: self.glow_radius, intensity: self.glow_intensity })
                                } else {
                                    None
                                };
                                let crt_opt = if self.crt_enabled {
                                    Some(VintageCrtSimulation {
                                        scanline_intensity: self.crt_scanline,
                                        barrel_curvature: self.crt_barrel,
                                        vignette_intensity: self.crt_vignette,
                                    })
                                } else {
                                    None
                                };
                                let blur_opt = if self.blur_enabled { Some(GaussianBlur::new(self.blur_sigma)) } else { None };
                                let sharpen_opt = if self.sharpen_enabled { Some(Sharpen::new(self.sharpen_amount, 1.0)) } else { None };
                                let chromatic_opt = if self.chromatic_enabled {
                                    Some(ChromaticAberration { shift_x: self.chromatic_shift, shift_y: 0, radial: self.chromatic_radial })
                                } else {
                                    None
                                };
                                let glitch_opt = if self.glitch_enabled {
                                    Some(JpegGlitchCore { mode: self.glitch_mode, intensity: self.glitch_intensity, block_size: 8, seed: 42 })
                                } else {
                                    None
                                };

                                use rayon::prelude::*;
                                let mut rendered: Vec<_> = frames_to_render
                                    .par_iter()
                                    .enumerate()
                                    .map(|(frame_idx, src_buf)| {
                                        let mut current = (**src_buf).clone();
                                        if let Some((pattern, intensity)) = temporal_noise {
                                            current = pattern.apply(&current, frame_idx as u32, intensity).unwrap_or(current);
                                        }
                                        let algo = create_algorithm(&algo_name).unwrap_or_else(|| Box::new(FloydSteinberg::default()));
                                        let mut out = algo.apply(&current, &palette).unwrap_or(current);
                                        if let Some(glow) = &glow_opt {
                                            out = glow.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(chroma) = &chromatic_opt {
                                            out = chroma.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(blur) = &blur_opt {
                                            out = blur.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(sharp) = &sharpen_opt {
                                            out = sharp.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(crt) = &crt_opt {
                                            out = crt.apply(&out).unwrap_or(out);
                                        }
                                        if let Some(glitch) = &glitch_opt {
                                            out = glitch.apply(&out).unwrap_or(out);
                                        }

                                        let delay_ms = delays.get(frame_idx).copied().unwrap_or(default_delay_ms);
                                        (out, delay_ms)
                                    })
                                    .collect();

                                if self.temporal_coherence_enabled {
                                    let raw_sources: Vec<ImageBuffer> = frames_to_render.iter().map(|b| (**b).clone()).collect();
                                    let dithered: Vec<ImageBuffer> = rendered.iter().map(|(b, _)| b.clone()).collect();
                                    let filter = TemporalCoherenceFilter::new(TemporalCoherenceConfig {
                                        damping: self.temporal_coherence_damping,
                                        motion_threshold: self.temporal_coherence_threshold,
                                    });
                                    if let Ok(filtered) = filter.apply_sequence(&raw_sources, &dithered) {
                                        for (i, f) in filtered.into_iter().enumerate() {
                                            if i < rendered.len() {
                                                rendered[i].0 = f;
                                            }
                                        }
                                    }
                                }

                                for (buf, delay_ms) in rendered {
                                    let mut single_webp = Vec::new();
                                    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut single_webp);
                                    if let Ok(()) = encoder.encode(&buf.data, buf.width, buf.height, image::ExtendedColorType::Rgb8)
                                        && let Some(subchunk) = WebpAnimationMuxer::extract_image_chunk(&single_webp)
                                    {
                                        muxer.add_frame(WebpAnimFrame {
                                            duration_ms: delay_ms,
                                            x: 0,
                                            y: 0,
                                            width: buf.width,
                                            height: buf.height,
                                            payload: subchunk.to_vec(),
                                        });
                                    }
                                }

                                if let Ok(anim_bytes) = muxer.build()
                                    && let Ok(()) = std::fs::write(&path, anim_bytes)
                                {
                                    self.status_text = format!("Exported animated WebP ({num_frames} frames)");
                                }
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Black Vector SVG...").clicked() {
                        let fname = self.suggested_filename("svg");
                        let save_target = self.rendered_image.as_ref().and_then(|img| {
                            rfd::FileDialog::new().set_file_name(&fname).add_filter("SVG Vector", &["svg"]).save_file().map(|path| (img, path))
                        });
                        if let Some((img, path)) = save_target {
                            let isolator = BlackVectorIsolation::default();
                            if let Ok(svg) = isolator.trace_to_svg(img) {
                                let _ = std::fs::write(path, svg);
                                self.status_text = "Exported black vector SVG".into();
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Multi-Color SVG...").clicked() {
                        let fname = self.suggested_filename("svg");
                        let save_target = self.rendered_image.as_ref().and_then(|img| {
                            rfd::FileDialog::new().set_file_name(&fname).add_filter("Layered SVG", &["svg"]).save_file().map(|path| (img, path))
                        });
                        if let Some((img, path)) = save_target {
                            let separator = MultiColorSvgSeparation::default();
                            if let Ok(svg) = separator.trace_palette_layers(img, &self.active_palette) {
                                let _ = std::fs::write(path, svg);
                                self.status_text = "Exported multi-color SVG layers".into();
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export CNC / Plotter G-Code...").clicked() {
                        let fname = self.suggested_filename("gcode");
                        let save_target = self.rendered_image.as_ref().and_then(|img| {
                            rfd::FileDialog::new()
                                .set_file_name(&fname)
                                .add_filter("CNC G-Code", &["gcode", "nc"])
                                .save_file()
                                .map(|path| (img, path))
                        });
                        if let Some((img, path)) = save_target {
                            let config = GCodeConfig::default();
                            if let Ok(gcode) = GCodeExporter::generate_gcode(img, &config) {
                                let _ = std::fs::write(path, gcode);
                                self.status_text = "Exported plotter G-code with TSP sorting".into();
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Smooth Bézier Spline SVG...").clicked() {
                        let fname = self.suggested_filename("svg");
                        let save_target = self.rendered_image.as_ref().and_then(|img| {
                            rfd::FileDialog::new().set_file_name(&fname).add_filter("Smooth Bézier SVG", &["svg"]).save_file().map(|path| (img, path))
                        });
                        if let Some((img, path)) = save_target
                            && let Ok(svg) = export_smooth_bezier_svg(img, 128, 0.5)
                        {
                            let _ = std::fs::write(path, svg);
                            self.status_text = "Exported smooth Bézier spline SVG".into();
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Silkscreen Print Plates (.svg)...").clicked() {
                        let pick_target = self.rendered_image.as_ref().and_then(|img| rfd::FileDialog::new().pick_folder().map(|dir| (img, dir)));
                        if let Some((img, dir)) = pick_target {
                            let sep = SpotColorPlateSeparation::default();
                            if let Ok(plates) = sep.generate_all_plates(img, &self.active_palette) {
                                for (filename, content) in &plates {
                                    let _ = std::fs::write(dir.join(filename), content);
                                }
                                self.status_text = format!("Exported {} spot color plates to {}", plates.len(), dir.display());
                            }
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export CMYK Process Separation Plates (.svg)...").clicked() {
                        let pick_target = self
                            .source_image
                            .as_ref()
                            .map(|s| (**s).clone())
                            .or_else(|| self.rendered_image.clone())
                            .and_then(|img| rfd::FileDialog::new().pick_folder().map(|dir| (img, dir)));
                        if let Some((img, dir)) = pick_target {
                            let cmyk = CmykHalftone { dot_shape: self.cmyk_dot_shape, dot_gain: self.cmyk_dot_gain, ..Default::default() };
                            if let Ok(seps) = cmyk.generate_separations(&img)
                                && let Ok(plates) = seps.export_all_svg_plates(&cmyk, 30.0, 1.0)
                            {
                                for (filename, content) in &plates {
                                    let _ = std::fs::write(dir.join(filename), content);
                                }
                                self.status_text = format!("Exported 4 CMYK process separation plates to {}", dir.display());
                            }
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Batch Processing Studio...").clicked() {
                        self.show_batch_studio = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Import Palette...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Palette", &["json", "hex", "gpl", "pal", "act"]).pick_file()
                            && let Ok(pal) = Palette::import_from_file(&path)
                        {
                            self.selected_palette_name = pal.name.clone();
                            self.active_palette = pal;
                            self.trigger_render();
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Active Palette...").clicked() {
                        let fname = format!("{}.gpl", self.active_palette.name.to_lowercase().replace(' ', "-"));
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&fname)
                            .add_filter("GIMP Palette (.gpl)", &["gpl"])
                            .add_filter("Color Table (.act)", &["act"])
                            .add_filter("Hex Palette (.hex)", &["hex"])
                            .add_filter("JASC-PAL (.pal)", &["pal"])
                            .add_filter("JSON Palette (.json)", &["json"])
                            .save_file()
                            && let Ok(()) = self.active_palette.export_to_file(&path)
                        {
                            self.status_text = format!("Exported palette to {}", path.display());
                        }
                        ui.close_menu();
                    }

                    if ui.button("Extract Palette (Median Cut 16)...").clicked() {
                        if let Some(ref src) = self.source_image
                            && let Ok(pal) = Palette::from_image_median_cut("Median Cut 16", src, 16)
                        {
                            self.selected_palette_name = "Extracted Median Cut".into();
                            self.active_palette = pal;
                            self.trigger_render();
                        }
                        ui.close_menu();
                    }

                    if ui.button("Extract Palette (K-Means 16)...").clicked() {
                        if let Some(ref src) = self.source_image
                            && let Ok(pal) = Palette::from_image_kmeans("K-Means 16", src, 16, 10)
                        {
                            self.selected_palette_name = "Extracted K-Means".into();
                            self.active_palette = pal;
                            self.trigger_render();
                        }
                        ui.close_menu();
                    }

                    if ui.button("Perceptual Color Harmonies (OKLCH)...").clicked() {
                        self.show_harmonies_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo (Ctrl+Z)").clicked() {
                        self.undo();
                        ui.close_menu();
                    }
                    if ui.button("Redo (Ctrl+Shift+Z)").clicked() {
                        self.redo();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Transform", |ui| {
                    if ui.button("⟳ Rotate 90° Clockwise").clicked() {
                        self.transform_rotate_90_cw();
                        ui.close_menu();
                    }
                    if ui.button("↻ Rotate 180°").clicked() {
                        self.transform_rotate_180();
                        ui.close_menu();
                    }
                    if ui.button("⟲ Rotate 90° Counter-Clockwise").clicked() {
                        self.transform_rotate_270_cw();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("⇄ Flip Horizontal").clicked() {
                        self.transform_flip_horizontal();
                        ui.close_menu();
                    }
                    if ui.button("⇅ Flip Vertical").clicked() {
                        self.transform_flip_vertical();
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label("Integer Nearest-Neighbor Scale:");
                    if ui.button("2x Integer Scale").clicked() {
                        self.transform_scale_integer(2);
                        ui.close_menu();
                    }
                    if ui.button("4x Integer Scale").clicked() {
                        self.transform_scale_integer(4);
                        ui.close_menu();
                    }
                    if ui.button("8x Integer Scale").clicked() {
                        self.transform_scale_integer(8);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("✂ Crop Workshop...").clicked() {
                        if let Some(src) = &self.source_image {
                            self.crop_x = 0;
                            self.crop_y = 0;
                            self.crop_width = src.width;
                            self.crop_height = src.height;
                            self.crop_dialog_open = true;
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Presets", |ui| {
                    if ui.button("🖼 Visual Preset Gallery...").clicked() {
                        self.show_preset_gallery = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label("Curated Profiles:");
                    for p in StudioPreset::bundled_presets() {
                        if ui.button(&p.name).clicked() {
                            self.record_undo_state();
                            self.apply_preset(&p);
                            self.status_text = format!("Loaded preset: {}", p.name);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Import Preset (.dcpreset)...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("DitherCraft Preset", &["dcpreset", "json"]).pick_file()
                            && let Ok(data) = std::fs::read_to_string(path)
                            && let Ok(preset) = serde_json::from_str::<StudioPreset>(&data)
                        {
                            self.record_undo_state();
                            self.apply_preset(&preset);
                            self.status_text = format!("Imported preset: {}", preset.name);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Export Active Preset...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("DitherCraft Preset", &["dcpreset", "json"]).save_file() {
                            let preset = self.capture_preset("Exported Preset");
                            if let Ok(json) = serde_json::to_string_pretty(&preset) {
                                let _ = std::fs::write(path, json);
                                self.status_text = "Preset saved".into();
                            }
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label("Instant A/B Snapshots (F5-F8):");
                    for (idx, label) in ['A', 'B', 'C', 'D'].iter().enumerate() {
                        let is_stored = self.snapshots[idx].is_some();
                        let title = if let Some(ref s) = self.snapshots[idx] {
                            format!("Recall Snapshot {label} (F{}) — {} ({})", idx + 5, s.algo, s.palette_name)
                        } else {
                            format!("Snapshot {label} [Empty] — Store Current (Shift+F{})", idx + 5)
                        };
                        if ui.button(title).clicked() {
                            if is_stored {
                                if let Some(snap) = self.snapshots[idx].clone() {
                                    self.record_undo_state();
                                    self.apply_preset(&snap);
                                    self.status_text = format!("Recalled Snapshot {label}: {} ({})", snap.algo, snap.palette_name);
                                    self.trigger_render();
                                }
                            } else {
                                let snap = self.capture_preset(format!("Snapshot {label}"));
                                self.status_text = format!("Stored Snapshot {label}: {} ({})", snap.algo, snap.palette_name);
                                self.snapshots[idx] = Some(snap);
                            }
                            ui.close_menu();
                        }
                    }
                });

                ui.menu_button("Theme", |ui| {
                    for t in [
                        StudioTheme::StudioDark,
                        StudioTheme::AmberCrt,
                        StudioTheme::CyberMatrix,
                        StudioTheme::VaporSlate,
                        StudioTheme::IndustrialHighContrast,
                    ] {
                        if ui.selectable_label(self.theme == t, t.name()).clicked() {
                            self.theme = t;
                            ui.close_menu();
                        }
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_telemetry_hud, "Pipeline Telemetry HUD");
                    ui.checkbox(&mut self.show_loupe, "Pixel Magnifier Loupe (L)");
                    ui.checkbox(&mut self.show_pixel_grid, "Pixel Grid (>=800%)");
                    ui.checkbox(&mut self.split_view_enabled, "Split-View Comparison");
                    ui.checkbox(&mut self.debounce_mode, "Debounce Slider Preview");
                    ui.separator();
                    ui.label("Color Vision Deficiency (CVD):");
                    for mode in CvdMode::ALL {
                        if ui.selectable_label(self.cvd_mode == mode, mode.name()).clicked() {
                            self.cvd_mode = mode;
                            self.trigger_render();
                        }
                    }
                    ui.separator();
                    ui.label("Retro Aspect Ratio Framing Guide:");
                    for guide in RetroAspectGuide::ALL {
                        ui.selectable_value(&mut self.aspect_guide, guide, guide.label());
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard Shortcuts Cheatsheet (F1 / ?)...").clicked() {
                        self.show_shortcuts_dialog = true;
                        ui.close_menu();
                    }
                    if ui.button("About DitherCraft...").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // About Dialog Modal
        if self.show_about {
            egui::Window::new("About DitherCraft").collapsible(false).resizable(false).open(&mut self.show_about).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(ref logo) = self.logo_texture {
                        ui.image((logo.id(), egui::vec2(96.0, 96.0)));
                    }
                    ui.add_space(4.0);
                    ui.heading("DitherCraft");
                    ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
                    ui.label("Studio-Grade Retro Graphics Workstation in Pure Rust");
                    ui.add_space(8.0);
                    ui.label("Clean-room implementation with zero proprietary assets.");
                    ui.label("Dual-licensed under MIT / Apache-2.0.");
                });
            });
        }

        // Keyboard Shortcuts Cheatsheet Modal
        if self.show_shortcuts_dialog {
            let mut is_open = true;
            let mut close_requested = false;
            egui::Window::new("Keyboard Shortcuts Cheatsheet").collapsible(false).resizable(false).open(&mut is_open).show(ctx, |ui| {
                ui.heading("DitherCraft Studio Shortcuts");
                ui.add_space(6.0);
                egui::Grid::new("shortcuts_grid").striped(true).spacing([28.0, 6.0]).show(ui, |ui| {
                    ui.strong("Action");
                    ui.strong("Shortcut");
                    ui.end_row();

                    ui.label("Undo");
                    ui.monospace("Ctrl+Z");
                    ui.end_row();

                    ui.label("Redo");
                    ui.monospace("Ctrl+Shift+Z / Ctrl+Y");
                    ui.end_row();

                    ui.label("Toggle Precision Magnifier Loupe");
                    ui.monospace("L");
                    ui.end_row();

                    ui.label("Recall A/B Snapshot A – D");
                    ui.monospace("F5 / F6 / F7 / F8");
                    ui.end_row();

                    ui.label("Store Current to Snapshot A – D");
                    ui.monospace("Shift + F5..F8");
                    ui.end_row();

                    ui.label("Pan Canvas Viewport");
                    ui.monospace("Space / Alt + Drag, Middle Drag");
                    ui.end_row();

                    ui.label("Zoom In / Out");
                    ui.monospace("Mouse Scroll Wheel");
                    ui.end_row();

                    ui.label("Reset Zoom (1:1 100%)");
                    ui.monospace("HUD [1:1]");
                    ui.end_row();

                    ui.label("Toggle Pixel Grid (at >=800%)");
                    ui.monospace("HUD [Grid]");
                    ui.end_row();

                    ui.label("Toggle Shortcuts Cheatsheet");
                    ui.monospace("F1 / ?");
                    ui.end_row();

                    ui.label("Workstation Pointer Tool");
                    ui.monospace("V");
                    ui.end_row();

                    ui.label("Brush Mask Tool");
                    ui.monospace("B");
                    ui.end_row();

                    ui.label("Eraser Mask Tool");
                    ui.monospace("E");
                    ui.end_row();

                    ui.label("Rect Marquee Selection");
                    ui.monospace("M");
                    ui.end_row();

                    ui.label("Magic Wand Selection");
                    ui.monospace("W");
                    ui.end_row();

                    ui.label("Toggle Quick Mask Overlay");
                    ui.monospace("Q");
                    ui.end_row();

                    ui.label("Select All Canvas");
                    ui.monospace("Ctrl+A");
                    ui.end_row();

                    ui.label("Deselect Selection");
                    ui.monospace("Ctrl+D");
                    ui.end_row();

                    ui.label("Invert Selection");
                    ui.monospace("Ctrl+Shift+I");
                    ui.end_row();
                });
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    close_requested = true;
                }
            });
            if close_requested {
                is_open = false;
            }
            self.show_shortcuts_dialog = is_open;
        }

        // Canvas Geometry Crop Workshop Modal Dialog
        if self.crop_dialog_open {
            let mut is_open = true;
            let mut apply_crop = false;
            let mut cancel_crop = false;
            egui::Window::new("Canvas Geometry — Crop Workshop").collapsible(false).resizable(false).open(&mut is_open).show(ctx, |ui| {
                if let Some(src) = &self.source_image {
                    ui.heading("Interactive Canvas Crop");
                    ui.label(format!("Source Dimensions: {} x {} px", src.width, src.height));
                    ui.add_space(6.0);

                    egui::Grid::new("crop_inputs_grid").spacing([16.0, 6.0]).show(ui, |ui| {
                        ui.label("Origin X:");
                        let max_x = src.width.saturating_sub(1);
                        ui.add(egui::DragValue::new(&mut self.crop_x).range(0..=max_x));
                        ui.end_row();

                        ui.label("Origin Y:");
                        let max_y = src.height.saturating_sub(1);
                        ui.add(egui::DragValue::new(&mut self.crop_y).range(0..=max_y));
                        ui.end_row();

                        ui.label("Crop Width:");
                        let max_w = src.width.saturating_sub(self.crop_x).max(1);
                        ui.add(egui::DragValue::new(&mut self.crop_width).range(1..=max_w));
                        ui.end_row();

                        ui.label("Crop Height:");
                        let max_h = src.height.saturating_sub(self.crop_y).max(1);
                        ui.add(egui::DragValue::new(&mut self.crop_height).range(1..=max_h));
                        ui.end_row();
                    });

                    ui.separator();
                    ui.label("Retro Display Aspect Ratio Presets:");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Game Boy (160×144)").clicked() {
                            self.crop_width = 160.min(src.width);
                            self.crop_height = 144.min(src.height);
                        }
                        if ui.button("NES / SNES (256×224)").clicked() {
                            self.crop_width = 256.min(src.width);
                            self.crop_height = 224.min(src.height);
                        }
                        if ui.button("CGA / EGA (320×200)").clicked() {
                            self.crop_width = 320.min(src.width);
                            self.crop_height = 200.min(src.height);
                        }
                        if ui.button("Arcade CRT (224×288)").clicked() {
                            self.crop_width = 224.min(src.width);
                            self.crop_height = 288.min(src.height);
                        }
                        if ui.button("Square (1:1)").clicked() {
                            let side = src.width.min(src.height);
                            self.crop_width = side;
                            self.crop_height = side;
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Apply Crop").clicked() {
                            apply_crop = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel_crop = true;
                        }
                    });
                }
            });

            if apply_crop {
                let (x, y, w, h) = (self.crop_x, self.crop_y, self.crop_width, self.crop_height);
                self.transform_crop(x, y, w, h);
                self.crop_dialog_open = false;
            } else if cancel_crop || !is_open {
                self.crop_dialog_open = false;
            }
        }

        // Visual Preset Gallery Modal Dialog
        if self.show_preset_gallery {
            let mut is_open = true;
            let mut preset_to_apply = None;
            egui::Window::new("Visual Preset Gallery").resizable(true).default_width(620.0).default_height(420.0).open(&mut is_open).show(
                ctx,
                |ui| {
                    ui.heading("Curated Studio Profiles");
                    ui.label("Preview and instantly apply signature retro color grading, CRT simulation, and dither aesthetics.");
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for p in StudioPreset::bundled_presets() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.strong(&p.name);
                                        ui.label(format!("• Algorithm: {} | Palette: {}", p.algo, p.palette_name));
                                        ui.horizontal(|ui| {
                                            if p.crt_enabled {
                                                ui.colored_label(Color32::from_rgb(255, 176, 0), "[CRT Scanlines]");
                                            }
                                            if p.glow_enabled {
                                                ui.colored_label(Color32::from_rgb(130, 200, 255), "[Epsilon Bloom]");
                                            }
                                            if p.chromatic_enabled {
                                                ui.colored_label(Color32::from_rgb(255, 120, 180), "[Chromatic]");
                                            }
                                            if p.sharpen_enabled {
                                                ui.colored_label(Color32::from_rgb(180, 255, 120), "[Sharpen]");
                                            }
                                            if p.temporal_enabled {
                                                ui.colored_label(Color32::from_rgb(220, 160, 255), "[Temporal Noise]");
                                            }
                                        });
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Apply Preset").clicked() {
                                            preset_to_apply = Some(p.clone());
                                        }
                                    });
                                });
                            });
                            ui.add_space(4.0);
                        }
                    });
                },
            );

            if let Some(p) = preset_to_apply {
                self.record_undo_state();
                self.apply_preset(&p);
                self.status_text = format!("Applied preset: {}", p.name);
                is_open = false;
            }
            self.show_preset_gallery = is_open;
        }

        // OKLCH Perceptual Color Harmonies Modal Dialog
        if self.show_harmonies_dialog {
            let mut is_open = true;
            let mut close_requested = false;
            let mut apply_requested = false;
            egui::Window::new("OKLCH Perceptual Color Harmonies").resizable(false).default_width(440.0).open(&mut is_open).show(ctx, |ui| {
                ui.heading("Generate Balanced Retro Palette");
                ui.label("Synthesizes harmonic swatches across perceptual polar OKLCH color space.");
                ui.add_space(8.0);

                // Base color picker
                ui.horizontal(|ui| {
                    ui.label("Base Key Color:");
                    let rgb = self.harmony_base_rgb;
                    let mut color32 = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                    if ui.color_edit_button_srgba(&mut color32).changed() {
                        self.harmony_base_rgb = [color32.r(), color32.g(), color32.b()];
                    }
                    ui.monospace(format!("#{:02X}{:02X}{:02X}", self.harmony_base_rgb[0], self.harmony_base_rgb[1], self.harmony_base_rgb[2]));
                });

                // Harmony mode selection
                ui.horizontal(|ui| {
                    ui.label("Harmony Scheme:");
                    egui::ComboBox::from_id_salt("harmony_mode_picker").selected_text(self.harmony_mode.name()).show_ui(ui, |ui| {
                        for h in [
                            ColorHarmony::Complementary,
                            ColorHarmony::Triadic,
                            ColorHarmony::Tetradic,
                            ColorHarmony::Analogous,
                            ColorHarmony::SplitComplementary,
                            ColorHarmony::Monochromatic,
                        ] {
                            ui.selectable_value(&mut self.harmony_mode, h, h.name());
                        }
                    });
                });

                // Swatch count
                ui.add(egui::Slider::new(&mut self.harmony_count, 2..=16).text("Color Count"));

                ui.add_space(8.0);
                ui.label("Generated Swatches:");

                // Generate preview palette
                let base_color = RgbColor::new(self.harmony_base_rgb[0], self.harmony_base_rgb[1], self.harmony_base_rgb[2]);
                let preview_palette = Palette::from_harmony(base_color, self.harmony_mode, self.harmony_count);

                ui.horizontal(|ui| {
                    for swatch in &preview_palette.swatches {
                        let c = swatch.color;
                        let c32 = Color32::from_rgb(c.r, c.g, c.b);
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 3.0, c32);
                    }
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Apply as Active Palette").clicked() {
                        apply_requested = true;
                    }
                    if ui.button("Export Palette...").clicked() {
                        let fname = format!("harmony_{}.gpl", self.harmony_mode.name().to_lowercase().replace(' ', "-"));
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&fname)
                            .add_filter("GIMP Palette (.gpl)", &["gpl"])
                            .add_filter("JSON Palette (.json)", &["json"])
                            .add_filter("Hex Palette (.hex)", &["hex"])
                            .save_file()
                            && preview_palette.export_to_file(&path).is_ok()
                        {
                            self.status_text = format!("Exported harmony palette to {}", path.display());
                        }
                    }
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });

                if apply_requested {
                    self.record_undo_state();
                    self.active_palette = preview_palette;
                    self.selected_palette_name = format!("Harmony - {}", self.harmony_mode.name());
                    self.status_text = format!("Generated OKLCH {} harmony palette ({} colors)", self.harmony_mode.name(), self.harmony_count);
                    self.trigger_render();
                    close_requested = true;
                }
            });

            if close_requested {
                is_open = false;
            }
            self.show_harmonies_dialog = is_open;
        }

        // Telemetry HUD Window
        if self.show_telemetry_hud {
            egui::Window::new("Pipeline Telemetry").resizable(false).open(&mut self.show_telemetry_hud).show(ctx, |ui| {
                ui.heading("Render Stage Breakdown");
                ui.label(format!("Algorithm: {}", self.selected_algo));
                ui.label(format!("Active Gen: #{}", self.render_generation.load(Ordering::Relaxed)));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("• Kernel Dither:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.monospace(format!("{:.2} ms", self.telemetry_dither_ms));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("• Effects Stack:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.monospace(format!("{:.2} ms", self.telemetry_effects_ms));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("• Swatch Histogram:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.monospace(format!("{:.2} ms", self.telemetry_histogram_ms));
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong("Total Latency:");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.strong(format!("{:.2} ms", self.render_latency_ms));
                    });
                });
            });
        }

        // Batch Processing Studio Modal
        if self.show_batch_studio {
            egui::Window::new("Batch Processing Studio").resizable(true).default_width(500.0).open(&mut self.show_batch_studio).show(ctx, |ui| {
                ui.heading("Batch Processing Queue");
                ui.label("Process a folder of images using the active algorithm and palette.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Input Folder:");
                    ui.text_edit_singleline(&mut self.batch_input_dir);
                    if ui.button("Browse...").clicked()
                        && let Some(folder) = rfd::FileDialog::new().pick_folder()
                    {
                        self.batch_input_dir = folder.display().to_string();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Output Folder:");
                    ui.text_edit_singleline(&mut self.batch_output_dir);
                    if ui.button("Browse...").clicked()
                        && let Some(folder) = rfd::FileDialog::new().pick_folder()
                    {
                        self.batch_output_dir = folder.display().to_string();
                    }
                });

                ui.separator();
                ui.label(format!("Active Algorithm: {}", self.selected_algo));
                ui.label(format!("Active Palette: {} ({} colors)", self.active_palette.name, self.active_palette.len()));
                ui.add_space(4.0);

                if self.batch_is_running {
                    ui.add(egui::ProgressBar::new(self.batch_progress).show_percentage());
                    ui.label(&self.batch_status_msg);
                } else {
                    if ui.button("▶ Run Batch Process").clicked() {
                        let in_path = std::path::PathBuf::from(&self.batch_input_dir);
                        let out_path = std::path::PathBuf::from(&self.batch_output_dir);
                        if in_path.is_dir() && !self.batch_output_dir.is_empty() {
                            self.batch_status_msg = "Batch processing running in background...".into();
                            self.batch_progress = 1.0;

                            let algo_name = self.selected_algo.clone();
                            let pal = self.active_palette.clone();
                            let ctx_clone = ctx.clone();

                            rayon::spawn(move || {
                                let _ = std::fs::create_dir_all(&out_path);
                                let mut entries = Vec::new();
                                if let Ok(rd) = std::fs::read_dir(&in_path) {
                                    for e in rd.flatten() {
                                        let p = e.path();
                                        if let Some(ext) = p.extension().and_then(|x| x.to_str()).map(str::to_lowercase)
                                            && ["png", "jpg", "jpeg", "webp", "bmp"].contains(&ext.as_str())
                                        {
                                            entries.push(p);
                                        }
                                    }
                                }

                                if !entries.is_empty() {
                                    let algo = create_algorithm(&algo_name)
                                        .unwrap_or_else(|| create_algorithm("Floyd-Steinberg").expect("default algorithm"));
                                    for file_path in entries {
                                        if let Ok(reader) = image::ImageReader::open(&file_path)
                                            && let Ok(img) = reader.decode()
                                        {
                                            let rgb = img.to_rgb8();
                                            let (w, h) = rgb.dimensions();
                                            if let Ok(buf) = ImageBuffer::from_raw(w, h, rgb.into_raw())
                                                && let Ok(out_buf) = algo.apply(&buf, &pal)
                                                && let Some(fname) = file_path.file_name()
                                            {
                                                let dest = out_path.join(fname);
                                                let _ =
                                                    image::save_buffer(dest, &out_buf.data, out_buf.width, out_buf.height, image::ColorType::Rgb8);
                                            }
                                        }
                                        ctx_clone.request_repaint();
                                    }
                                }
                            });
                        } else {
                            self.batch_status_msg = "Please select valid input and output directories".into();
                        }
                    }
                    ui.label(&self.batch_status_msg);
                }
            });
        }

        // Left Control Panel
        egui::SidePanel::left("control_panel").min_width(280.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref logo) = self.logo_texture {
                    ui.image((logo.id(), egui::vec2(24.0, 24.0)));
                }
                ui.heading("DitherCraft Studio");
            });
            ui.separator();

            let mut changed = false;
            let mut preview_changed = false;

            // Pre-Dither Tone Levels & Gamma
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Input Levels & Gamma:");
                    if ui.button("↺ Reset").on_hover_text("Reset Tone Levels (Black 0, White 255, Gamma 1.0)").clicked() {
                        self.levels_black = 0;
                        self.levels_white = 255;
                        self.levels_gamma = 1.0;
                        changed = true;
                    }
                });
                check_slider(ui, egui::Slider::new(&mut self.levels_black, 0..=254).text("Black Point"), &mut changed, &mut preview_changed);
                if self.levels_black >= self.levels_white {
                    self.levels_white = self.levels_black + 1;
                }
                check_slider(ui, egui::Slider::new(&mut self.levels_white, 1..=255).text("White Point"), &mut changed, &mut preview_changed);
                if self.levels_white <= self.levels_black {
                    self.levels_black = self.levels_white.saturating_sub(1);
                }
                check_slider(
                    ui,
                    egui::Slider::new(&mut self.levels_gamma, 0.1..=3.0).text("Midtone Gamma").logarithmic(true),
                    &mut changed,
                    &mut preview_changed,
                );
            });

            // Workstation Tools, Masking & Selection
            ui.group(|ui| {
                ui.label("Workstation Tools & Masking:");
                ui.horizontal_wrapped(|ui| {
                    for tool in WorkstationTool::ALL {
                        if ui.selectable_label(self.active_tool == tool, tool.label()).clicked() {
                            self.active_tool = tool;
                        }
                    }
                });

                match self.active_tool {
                    WorkstationTool::BrushMask | WorkstationTool::EraserMask => {
                        ui.horizontal(|ui| {
                            ui.label("Radius:");
                            ui.add(egui::Slider::new(&mut self.brush_radius, 1.0..=128.0).text("px"));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Falloff:");
                            ui.add(egui::Slider::new(&mut self.brush_falloff, 0.0..=1.0).text("Hardness"));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Linear Grad").on_hover_text("Apply procedural linear gradient to mask").clicked()
                                && let Some(ref mut m) = self.mask
                            {
                                m.apply_linear_gradient(0.0, 0.0, m.width as f32, 0.0);
                                if self.mask_blend_enabled {
                                    changed = true;
                                } else {
                                    self.update_texture(ctx);
                                }
                            }
                            if ui.button("Radial Grad").on_hover_text("Apply procedural radial gradient to mask").clicked()
                                && let Some(ref mut m) = self.mask
                            {
                                let cx = m.width as f32 * 0.5;
                                let cy = m.height as f32 * 0.5;
                                m.apply_radial_gradient(cx, cy, 0.0, cx.min(cy));
                                if self.mask_blend_enabled {
                                    changed = true;
                                } else {
                                    self.update_texture(ctx);
                                }
                            }
                            if ui.button("Invert").on_hover_text("Invert alpha mask").clicked()
                                && let Some(ref mut m) = self.mask
                            {
                                m.invert();
                                if self.mask_blend_enabled {
                                    changed = true;
                                } else {
                                    self.update_texture(ctx);
                                }
                            }
                            if ui.button("Clear").on_hover_text("Clear mask to 0").clicked()
                                && let Some(ref mut m) = self.mask
                            {
                                m.clear();
                                if self.mask_blend_enabled {
                                    changed = true;
                                } else {
                                    self.update_texture(ctx);
                                }
                            }
                        });
                        if ui.checkbox(&mut self.quick_mask_active, "Quick Mask Overlay (Q)").changed() {
                            self.update_texture(ctx);
                        }
                    }
                    WorkstationTool::RectMarquee | WorkstationTool::EllipseMarquee | WorkstationTool::Lasso | WorkstationTool::MagicWand => {
                        ui.horizontal(|ui| {
                            ui.label("Mode:");
                            egui::ComboBox::from_id_salt("selection_mode_combo")
                                .selected_text(match self.selection_mode {
                                    SelectionMode::New => "New",
                                    SelectionMode::Add => "Add (+)",
                                    SelectionMode::Subtract => "Subtract (-)",
                                    SelectionMode::Intersect => "Intersect (∩)",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.selection_mode, SelectionMode::New, "New");
                                    ui.selectable_value(&mut self.selection_mode, SelectionMode::Add, "Add (+)");
                                    ui.selectable_value(&mut self.selection_mode, SelectionMode::Subtract, "Subtract (-)");
                                    ui.selectable_value(&mut self.selection_mode, SelectionMode::Intersect, "Intersect (∩)");
                                });
                        });
                        if self.active_tool == WorkstationTool::MagicWand {
                            ui.horizontal(|ui| {
                                ui.label("Wand Tolerance:");
                                ui.add(egui::Slider::new(&mut self.wand_tolerance, 0.01..=0.50).text("OKLab ΔE"));
                            });
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Select All").clicked() {
                                self.select_all();
                            }
                            if ui.button("Deselect").clicked() {
                                self.deselect();
                            }
                            if ui.button("Invert").clicked() {
                                self.invert_selection();
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Selection ➔ Mask").on_hover_text("Bake active marquee selection into mask").clicked() {
                                self.convert_selection_to_mask();
                                if self.mask_blend_enabled {
                                    changed = true;
                                } else {
                                    self.update_texture(ctx);
                                }
                            }
                            if ui.button("Mask ➔ Selection").on_hover_text("Extract marquee selection from mask threshold").clicked() {
                                self.convert_mask_to_selection();
                            }
                        });
                    }
                    WorkstationTool::Pointer => {
                        ui.label("Drag canvas to pan, scroll to zoom.");
                        if ui.checkbox(&mut self.quick_mask_active, "Quick Mask Overlay (Q)").changed() {
                            self.update_texture(ctx);
                        }
                    }
                }

                // Dual-Algorithm Spatial Blending
                ui.separator();
                if ui.checkbox(&mut self.mask_blend_enabled, "Dual-Algorithm Mask Blending").changed() {
                    changed = true;
                }
                if self.mask_blend_enabled {
                    ui.horizontal(|ui| {
                        ui.label("Secondary:");
                        egui::ComboBox::from_id_salt("secondary_algo_picker").width(180.0).selected_text(&self.secondary_algo).show_ui(ui, |ui| {
                            for info in dither_core::algorithms::ALL_ALGORITHMS.iter() {
                                if ui.selectable_value(&mut self.secondary_algo, info.name.to_string(), info.name).clicked() {
                                    changed = true;
                                }
                            }
                        });
                    });
                    ui.label(egui::RichText::new("Primary algo applies to Mask > 0; Secondary applies to Mask = 0.").italics().size(10.5));
                }
            });

            // Algorithm Search & Picker
            ui.group(|ui| {
                ui.label("Dithering Algorithm:");
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.algo_search_query);
                });

                let query = self.algo_search_query.to_lowercase();
                let filtered: Vec<_> = dither_core::algorithms::ALL_ALGORITHMS
                    .iter()
                    .filter(|info| query.is_empty() || info.name.to_lowercase().contains(&query))
                    .collect();

                egui::ComboBox::from_id_salt("algo_picker").width(240.0).selected_text(&self.selected_algo).show_ui(ui, |ui| {
                    if (query.is_empty() || "tonal band multi-dither".contains(&query))
                        && ui.selectable_value(&mut self.selected_algo, "Tonal Band Multi-Dither".into(), "Tonal Band Multi-Dither").clicked()
                    {
                        changed = true;
                    }
                    if (query.is_empty() || "custom threshold matrix".contains(&query))
                        && ui.selectable_value(&mut self.selected_algo, "Custom Threshold Matrix".into(), "Custom Threshold Matrix").clicked()
                    {
                        changed = true;
                    }
                    if (query.is_empty() || "morphogenesis (reaction-diffusion)".contains(&query))
                        && ui
                            .selectable_value(
                                &mut self.selected_algo,
                                "Morphogenesis (Reaction-Diffusion)".into(),
                                "Morphogenesis (Reaction-Diffusion)",
                            )
                            .clicked()
                    {
                        changed = true;
                    }
                    if (query.is_empty() || "vector flow field".contains(&query))
                        && ui.selectable_value(&mut self.selected_algo, "Vector Flow Field".into(), "Vector Flow Field").clicked()
                    {
                        changed = true;
                    }
                    for info in filtered {
                        if ui.selectable_value(&mut self.selected_algo, info.name.into(), info.name).clicked() {
                            changed = true;
                        }
                    }
                });

                if self.selected_algo.to_lowercase().contains("cmyk") {
                    ui.separator();
                    ui.label("CMYK Rosette Halftone Controls:");
                    ui.horizontal(|ui| {
                        ui.label("Dot Shape:");
                        egui::ComboBox::from_id_salt("cmyk_dot_shape")
                            .selected_text(match self.cmyk_dot_shape {
                                HalftoneDotShape::EuclideanRound => "Euclidean Round",
                                HalftoneDotShape::Elliptical => "Elliptical",
                                HalftoneDotShape::Diamond => "Diamond",
                                HalftoneDotShape::Line => "Line",
                            })
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut self.cmyk_dot_shape, HalftoneDotShape::EuclideanRound, "Euclidean Round").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut self.cmyk_dot_shape, HalftoneDotShape::Elliptical, "Elliptical").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut self.cmyk_dot_shape, HalftoneDotShape::Diamond, "Diamond").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut self.cmyk_dot_shape, HalftoneDotShape::Line, "Line").clicked() {
                                    changed = true;
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Dot Gain:");
                        let slider = egui::Slider::new(&mut self.cmyk_dot_gain, -0.2..=0.4)
                            .text("% gain")
                            .custom_formatter(|n, _| format!("{:.0}%", n * 100.0));
                        check_slider(ui, slider, &mut changed, &mut preview_changed);
                    });
                }

                if self.selected_algo.to_lowercase().contains("tonal band") {
                    ui.separator();
                    ui.label("Tonal Band Multi-Dither Controls:");
                    ui.horizontal(|ui| {
                        ui.label("Shadows:");
                        egui::ComboBox::from_id_salt("tonal_shadow_algo").selected_text(&self.tonal_shadow_algo).show_ui(ui, |ui| {
                            for name in ["Bayer 8x8", "Bayer 4x4", "Floyd-Steinberg", "Atkinson", "Stucki", "Jarvis-Judice-Ninke"] {
                                if ui.selectable_value(&mut self.tonal_shadow_algo, name.into(), name).clicked() {
                                    changed = true;
                                }
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Midtones:");
                        egui::ComboBox::from_id_salt("tonal_mid_algo").selected_text(&self.tonal_mid_algo).show_ui(ui, |ui| {
                            for name in ["Floyd-Steinberg", "Atkinson", "Burkes", "Sierra-3", "Bayer 8x8", "Interleaved Gradient Noise"] {
                                if ui.selectable_value(&mut self.tonal_mid_algo, name.into(), name).clicked() {
                                    changed = true;
                                }
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Highlights:");
                        egui::ComboBox::from_id_salt("tonal_hi_algo").selected_text(&self.tonal_highlight_algo).show_ui(ui, |ui| {
                            for name in ["Atkinson", "Floyd-Steinberg", "Bayer 4x4", "Bayer 8x8", "Ostromoukhov", "Riemersma"] {
                                if ui.selectable_value(&mut self.tonal_highlight_algo, name.into(), name).clicked() {
                                    changed = true;
                                }
                            }
                        });
                    });
                    check_slider(
                        ui,
                        egui::Slider::new(&mut self.tonal_shadow_threshold, 10..=128).text("Shadow Split"),
                        &mut changed,
                        &mut preview_changed,
                    );
                    check_slider(
                        ui,
                        egui::Slider::new(&mut self.tonal_highlight_threshold, 129..=245).text("Highlight Split"),
                        &mut changed,
                        &mut preview_changed,
                    );
                    check_slider(ui, egui::Slider::new(&mut self.tonal_feather, 2..=48).text("Feathering"), &mut changed, &mut preview_changed);
                }

                if self.selected_algo.to_lowercase().contains("custom") {
                    ui.separator();
                    ui.label("Custom Matrix Workshop:");
                    ui.label(format!("Active Matrix: {} ({}x{})", self.custom_matrix.name, self.custom_matrix.width, self.custom_matrix.height));
                    ui.horizontal(|ui| {
                        if ui.button("Spiral 4x4").clicked() {
                            self.custom_matrix = CustomMatrixDither::default();
                            changed = true;
                        }
                        if ui.button("Checker 2x2").clicked()
                            && let Ok(m) = CustomMatrixDither::new("Checker 2x2", 2, 2, vec![64, 192, 192, 64])
                        {
                            self.custom_matrix = m;
                            changed = true;
                        }
                        if ui.button("Bayer 4x4").clicked() {
                            let bayer = vec![0, 128, 32, 160, 192, 64, 224, 96, 48, 176, 16, 144, 240, 112, 208, 80];
                            if let Ok(m) = CustomMatrixDither::new("Bayer 4x4 Variant", 4, 4, bayer) {
                                self.custom_matrix = m;
                                changed = true;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Import JSON...").clicked()
                            && let Some(path) = rfd::FileDialog::new().add_filter("JSON Matrix", &["json"]).pick_file()
                            && let Ok(json) = std::fs::read_to_string(&path)
                            && let Ok(m) = CustomMatrixDither::from_json(&json)
                        {
                            self.status_text = format!("Imported matrix: {} ({}x{})", m.name, m.width, m.height);
                            self.custom_matrix = m;
                            changed = true;
                        }
                        if ui.button("Export JSON...").clicked() {
                            let fname = format!("{}.json", self.custom_matrix.name.to_lowercase().replace(' ', "-"));
                            if let Some(path) = rfd::FileDialog::new().set_file_name(&fname).add_filter("JSON Matrix", &["json"]).save_file()
                                && let Ok(json) = self.custom_matrix.to_json()
                            {
                                let _ = std::fs::write(&path, json);
                                self.status_text = format!("Exported matrix to {}", path.display());
                            }
                        }
                    });
                }

                if self.selected_algo.to_lowercase().contains("morphogenesis") || self.selected_algo.to_lowercase().contains("reaction-diffusion") {
                    ui.separator();
                    ui.label("Reaction-Diffusion Morphogenesis:");
                    ui.horizontal(|ui| {
                        ui.label("Regime:");
                        egui::ComboBox::from_id_salt("rd_morphology")
                            .selected_text(match self.rd_morphology {
                                ReactionDiffusionMorphology::Labyrinth => "Labyrinth",
                                ReactionDiffusionMorphology::Spots => "Leopard Spots",
                                ReactionDiffusionMorphology::Solitons => "Soliton Wavelets",
                                ReactionDiffusionMorphology::PulsatingWaves => "Pulsating Waves",
                                ReactionDiffusionMorphology::Fingerprints => "Fingerprint Ridges",
                                ReactionDiffusionMorphology::Coral => "Branching Coral",
                                _ => "Labyrinth",
                            })
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut self.rd_morphology, ReactionDiffusionMorphology::Labyrinth, "Labyrinth").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut self.rd_morphology, ReactionDiffusionMorphology::Spots, "Leopard Spots").clicked() {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut self.rd_morphology, ReactionDiffusionMorphology::Solitons, "Soliton Wavelets").clicked() {
                                    changed = true;
                                }
                                if ui
                                    .selectable_value(&mut self.rd_morphology, ReactionDiffusionMorphology::PulsatingWaves, "Pulsating Waves")
                                    .clicked()
                                {
                                    changed = true;
                                }
                                if ui
                                    .selectable_value(&mut self.rd_morphology, ReactionDiffusionMorphology::Fingerprints, "Fingerprint Ridges")
                                    .clicked()
                                {
                                    changed = true;
                                }
                                if ui.selectable_value(&mut self.rd_morphology, ReactionDiffusionMorphology::Coral, "Branching Coral").clicked() {
                                    changed = true;
                                }
                            });
                    });
                    check_slider(ui, egui::Slider::new(&mut self.rd_steps, 10..=200).text("PDE Steps"), &mut changed, &mut preview_changed);
                }

                if self.selected_algo.to_lowercase().contains("flow field") || self.selected_algo.to_lowercase().contains("vector flow") {
                    ui.separator();
                    ui.label("Vector Flow Field Steered Controls:");
                    check_slider(ui, egui::Slider::new(&mut self.flow_scale, 0.005..=0.20).text("Noise Scale"), &mut changed, &mut preview_changed);
                    check_slider(ui, egui::Slider::new(&mut self.flow_swirl, 0.0..=5.0).text("Vortex Swirl"), &mut changed, &mut preview_changed);
                }
            });

            // Palette Controls & Color Science
            ui.group(|ui| {
                ui.label("Target Palette & Metric:");
                let palettes = ["Monochrome", "Macintosh 1-bit", "Game Boy", "CGA Mode 1", "Cyberpunk", "Apple II", "Commodore 64", "NES", "PICO-8"];
                egui::ComboBox::from_id_salt("palette_picker").selected_text(&self.selected_palette_name).show_ui(ui, |ui| {
                    for p in palettes {
                        if ui.selectable_label(self.selected_palette_name == p, p).clicked() {
                            self.selected_palette_name = p.into();
                            let metric = self.active_palette.metric;
                            self.active_palette = match p {
                                "Monochrome" => Palette::monochrome(),
                                "Macintosh 1-bit" => Palette::mac_1bit(),
                                "CGA Mode 1" => Palette::cga_mode_1(),
                                "Cyberpunk" => Palette::cyberpunk(),
                                "Apple II" => Palette::apple_ii(),
                                "Commodore 64" => Palette::commodore_64(),
                                "NES" => Palette::nes(),
                                "PICO-8" => Palette::pico_8(),
                                _ => Palette::game_boy(),
                            };
                            self.active_palette.metric = metric;
                            changed = true;
                        }
                    }
                });

                // Color Metric Selection
                egui::ComboBox::from_id_salt("color_metric_picker")
                    .selected_text(match self.active_palette.metric {
                        ColorMetric::Redmean => "Redmean (Perceptual)",
                        ColorMetric::EuclideanRgb => "Euclidean RGB",
                        ColorMetric::CieLab => "CIELAB ΔE*",
                        ColorMetric::Oklab => "OKLab (Perceptual)",
                        ColorMetric::Ciede2000 => "CIEDE2000 ΔE00",
                    })
                    .show_ui(ui, |ui| {
                        for (metric, label) in [
                            (ColorMetric::Oklab, "OKLab (Perceptual)"),
                            (ColorMetric::Ciede2000, "CIEDE2000 ΔE00"),
                            (ColorMetric::Redmean, "Redmean (Perceptual)"),
                            (ColorMetric::EuclideanRgb, "Euclidean RGB"),
                            (ColorMetric::CieLab, "CIELAB ΔE*"),
                        ] {
                            if ui.selectable_value(&mut self.active_palette.metric, metric, label).clicked() {
                                changed = true;
                            }
                        }
                    });

                ui.horizontal(|ui| {
                    if ui.button("Sort Lum").clicked() {
                        self.active_palette.sort_by_luminance(true);
                        changed = true;
                    }
                    if ui.button("Randomize").clicked() {
                        self.active_palette.randomize_unlocked(Instant::now().elapsed().as_nanos() as u64);
                        changed = true;
                    }
                    if ui.button("Prune Dead").on_hover_text("Prune unlocked swatches with <0.1% usage").clicked()
                        && !self.swatch_percentages.is_empty()
                        && self.active_palette.swatches.len() == self.swatch_percentages.len()
                    {
                        let mut pruned = Vec::new();
                        for (i, swatch) in self.active_palette.swatches.iter().enumerate() {
                            let pct = self.swatch_percentages.get(i).copied().unwrap_or(0.0);
                            if swatch.locked || pct >= 0.1 {
                                pruned.push(*swatch);
                            }
                        }
                        if !pruned.is_empty() && pruned.len() < self.active_palette.swatches.len() {
                            self.active_palette.swatches = pruned;
                            changed = true;
                        }
                    }
                });

                // Swatches list with live color pickers, locks, utilization, and add/remove buttons
                ui.label("Swatches & Live Palette Editing:");
                let mut lock_changed = false;
                let mut delete_idx = None;
                let num_swatches = self.active_palette.swatches.len();
                for (idx, swatch) in self.active_palette.swatches.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let mut rgb = [swatch.color.r, swatch.color.g, swatch.color.b];
                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                            swatch.color = RgbColor::new(rgb[0], rgb[1], rgb[2]);
                            lock_changed = true;
                        }
                        if ui.checkbox(&mut swatch.locked, "Lock").changed() {
                            lock_changed = true;
                        }
                        if let Some(&pct) = self.swatch_percentages.get(idx) {
                            ui.label(format!("{pct:4.1}%"));
                        }
                        if num_swatches > 1 && ui.small_button("✕").on_hover_text("Remove swatch").clicked() {
                            delete_idx = Some(idx);
                        }
                    });
                }
                if let Some(d) = delete_idx {
                    self.active_palette.swatches.remove(d);
                    changed = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("+ Add Swatch").clicked() {
                        self.active_palette.swatches.push(Swatch::new(RgbColor::new(128, 128, 128), false));
                        changed = true;
                    }
                    if ui.button("🎨 Harmonies...").on_hover_text("Generate balanced OKLCH perceptual harmonies").clicked() {
                        self.show_harmonies_dialog = true;
                    }
                    if ui.button("💾 Export...").clicked() {
                        let fname = format!("{}.gpl", self.active_palette.name.to_lowercase().replace(' ', "-"));
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&fname)
                            .add_filter("GIMP Palette (.gpl)", &["gpl"])
                            .add_filter("Color Table (.act)", &["act"])
                            .add_filter("Hex Palette (.hex)", &["hex"])
                            .add_filter("JASC-PAL (.pal)", &["pal"])
                            .add_filter("JSON Palette (.json)", &["json"])
                            .save_file()
                            && let Ok(()) = self.active_palette.export_to_file(&path)
                        {
                            self.status_text = format!("Exported palette to {}", path.display());
                        }
                    }
                });
                if lock_changed {
                    changed = true;
                }
            });

            // Tonal Cycling
            ui.group(|ui| {
                ui.label("Tonal Cycling:");
                check_slider(ui, egui::Slider::new(&mut self.tonal_cycle_step, 0..=8).text("Cycle Step"), &mut changed, &mut preview_changed);
            });

            // Phase 8.2: Draggable & Reorderable Downstream Effects Stack
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Downstream Effects Stack (Layers):");
                    if ui.button("🎲 Shuffle").on_hover_text("Randomize retro glitch, CRT, and bloom adjustments").clicked() {
                        self.shuffle_adjustments();
                        changed = true;
                    }
                    if ui.button("↺ Reset").on_hover_text("Reset stack to defaults").clicked() {
                        self.effect_stack = EffectStack::default_stack();
                        changed = true;
                    }
                });

                let mut move_up = None;
                let mut move_down = None;
                let mut duplicate_node = None;
                let mut delete_node = None;
                let num_nodes = self.effect_stack.nodes.len();

                for (idx, node) in self.effect_stack.nodes.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            changed |= ui.checkbox(&mut node.enabled, "").changed();
                            ui.strong(&node.name);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("🗑").on_hover_text("Delete layer").clicked() {
                                    delete_node = Some(idx);
                                }
                                if ui.small_button("📋").on_hover_text("Duplicate layer").clicked() {
                                    duplicate_node = Some(idx);
                                }
                                if idx + 1 < num_nodes && ui.small_button("⬇").on_hover_text("Move down").clicked() {
                                    move_down = Some(idx);
                                }
                                if idx > 0 && ui.small_button("⬆").on_hover_text("Move up").clicked() {
                                    move_up = Some(idx);
                                }
                            });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Blend:");
                            egui::ComboBox::from_id_salt(format!("blend_{}", node.id)).selected_text(node.blend_mode.name()).show_ui(ui, |ui| {
                                for mode in BlendMode::ALL {
                                    if ui.selectable_value(&mut node.blend_mode, mode, mode.name()).clicked() {
                                        changed = true;
                                    }
                                }
                            });
                            ui.label("α:");
                            check_slider(ui, egui::Slider::new(&mut node.opacity, 0.0..=1.0).show_value(true), &mut changed, &mut preview_changed);
                        });

                        // Layer-specific parameter editors
                        match &mut node.kind {
                            EffectKind::Glow(glow) => {
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut glow.threshold, 50..=250).text("Threshold"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                                check_slider(ui, egui::Slider::new(&mut glow.radius, 1..=12).text("Radius"), &mut changed, &mut preview_changed);
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut glow.intensity, 0.1..=2.0).text("Intensity"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                            }
                            EffectKind::Chromatic(chroma) => {
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut chroma.shift_x, -15..=15).text("Shift X"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                                changed |= ui.checkbox(&mut chroma.radial, "Radial Shift").changed();
                            }
                            EffectKind::Glitch(glitch) => {
                                egui::ComboBox::from_id_salt(format!("glitch_mode_{}", node.id))
                                    .selected_text(match glitch.mode {
                                        JpegSubMode::BlockShift => "Block Shift",
                                        JpegSubMode::ChannelSwap => "Channel Swap",
                                        JpegSubMode::ScanLineOffset => "Scan Line Offset",
                                        JpegSubMode::BlockScramble => "Block Scramble",
                                        JpegSubMode::InterlaceCorruption => "Interlace Corruption",
                                    })
                                    .show_ui(ui, |ui| {
                                        for (m, lbl) in [
                                            (JpegSubMode::BlockShift, "Block Shift"),
                                            (JpegSubMode::ChannelSwap, "Channel Swap"),
                                            (JpegSubMode::ScanLineOffset, "Scan Line Offset"),
                                            (JpegSubMode::BlockScramble, "Block Scramble"),
                                            (JpegSubMode::InterlaceCorruption, "Interlace Corruption"),
                                        ] {
                                            if ui.selectable_value(&mut glitch.mode, m, lbl).clicked() {
                                                changed = true;
                                            }
                                        }
                                    });
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut glitch.intensity, 0.05..=1.0).text("Intensity"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                            }
                            EffectKind::Crt(crt) => {
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut crt.scanline_intensity, 0.0..=1.0).text("Scanlines"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut crt.barrel_curvature, 0.0..=0.3).text("Barrel"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut crt.vignette_intensity, 0.0..=1.0).text("Vignette"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                            }
                            EffectKind::Blur(blur) => {
                                check_slider(ui, egui::Slider::new(&mut blur.sigma, 0.2..=5.0).text("Sigma"), &mut changed, &mut preview_changed);
                            }
                            EffectKind::Sharpen(sharpen) => {
                                check_slider(
                                    ui,
                                    egui::Slider::new(&mut sharpen.amount, 0.1..=3.0).text("Amount"),
                                    &mut changed,
                                    &mut preview_changed,
                                );
                            }
                            EffectKind::Lut3d(lut) => {
                                ui.label(format!("3D LUT: '{}' ({}³)", lut.title, lut.size));
                            }
                        }
                    });
                }

                if let Some(idx) = move_up {
                    self.effect_stack.move_up(idx);
                    changed = true;
                }
                if let Some(idx) = move_down {
                    self.effect_stack.move_down(idx);
                    changed = true;
                }
                if let Some(idx) = duplicate_node {
                    self.effect_stack.duplicate(idx);
                    changed = true;
                }
                if let Some(idx) = delete_node {
                    self.effect_stack.remove(idx);
                    changed = true;
                }

                // Add Effect Layer Dropdown
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("add_effect_dropdown").selected_text("➕ Add Effect Layer...").show_ui(ui, |ui| {
                        if ui.button("Epsilon Glow Bloom").clicked() {
                            self.effect_stack.add(EffectNode::new(0, "Glow Bloom", EffectKind::Glow(EpsilonGlow::default())));
                            changed = true;
                        }
                        if ui.button("Chromatic Aberration").clicked() {
                            self.effect_stack.add(EffectNode::new(0, "Chromatic Aberration", EffectKind::Chromatic(ChromaticAberration::default())));
                            changed = true;
                        }
                        if ui.button("JPEG Glitch Core").clicked() {
                            self.effect_stack.add(EffectNode::new(0, "JPEG Glitch", EffectKind::Glitch(JpegGlitchCore::default())));
                            changed = true;
                        }
                        if ui.button("Vintage CRT Simulation").clicked() {
                            self.effect_stack.add(EffectNode::new(0, "Vintage CRT", EffectKind::Crt(VintageCrtSimulation::default())));
                            changed = true;
                        }
                        if ui.button("Gaussian Blur").clicked() {
                            self.effect_stack.add(EffectNode::new(0, "Gaussian Blur", EffectKind::Blur(GaussianBlur::default())));
                            changed = true;
                        }
                        if ui.button("Sharpen (Unsharp Mask)").clicked() {
                            self.effect_stack.add(EffectNode::new(0, "Sharpen", EffectKind::Sharpen(Sharpen::default())));
                            changed = true;
                        }
                    });
                });
            });

            // Temporal Noise & Animation
            ui.group(|ui| {
                ui.label("Temporal Noise & Animation:");
                if ui.checkbox(&mut self.temporal_noise_enabled, "Live Noise Stream").changed() {
                    changed = true;
                }
                if self.temporal_noise_enabled {
                    egui::ComboBox::from_id_salt("temporal_pattern_picker")
                        .selected_text(match self.temporal_pattern {
                            TemporalNoisePattern::CrtScanBeamJitter => "CRT Scan Beam Jitter",
                            TemporalNoisePattern::PhosphorGrainHum => "Phosphor Grain Hum",
                            TemporalNoisePattern::InterlacedFieldShimmer => "Interlaced Field Shimmer",
                            TemporalNoisePattern::LcdSubpixelGridDrift => "LCD Grid Drift",
                            TemporalNoisePattern::VhsTapePhaseNoise => "VHS Phase Noise",
                            TemporalNoisePattern::ThermalAnalogDrift => "Thermal Analog Drift",
                            TemporalNoisePattern::StaticHashWhiteNoise => "Static Hash Noise",
                            TemporalNoisePattern::BayerPhaseStepping => "Bayer Phase Stepping",
                            TemporalNoisePattern::BlueNoiseTemporalJitter => "Blue Noise Jitter",
                        })
                        .show_ui(ui, |ui| {
                            for (pat, label) in [
                                (TemporalNoisePattern::CrtScanBeamJitter, "CRT Scan Beam Jitter"),
                                (TemporalNoisePattern::PhosphorGrainHum, "Phosphor Grain Hum"),
                                (TemporalNoisePattern::InterlacedFieldShimmer, "Interlaced Field Shimmer"),
                                (TemporalNoisePattern::LcdSubpixelGridDrift, "LCD Grid Drift"),
                                (TemporalNoisePattern::VhsTapePhaseNoise, "VHS Phase Noise"),
                                (TemporalNoisePattern::ThermalAnalogDrift, "Thermal Analog Drift"),
                                (TemporalNoisePattern::StaticHashWhiteNoise, "Static Hash Noise"),
                                (TemporalNoisePattern::BayerPhaseStepping, "Bayer Phase Stepping"),
                                (TemporalNoisePattern::BlueNoiseTemporalJitter, "Blue Noise Jitter"),
                            ] {
                                if ui.selectable_value(&mut self.temporal_pattern, pat, label).clicked() {
                                    changed = true;
                                }
                            }
                        });

                    check_slider(
                        ui,
                        egui::Slider::new(&mut self.temporal_intensity, 0.05..=1.0).text("Noise Intensity"),
                        &mut changed,
                        &mut preview_changed,
                    );
                }
            });

            // Debounce and wheel settings
            ui.checkbox(&mut self.debounce_mode, "Debounce Rendering");
            ui.checkbox(&mut self.fine_wheel_increment, "Fine Wheel Increment");

            if changed {
                self.record_undo_state();
                self.trigger_render();
            } else if preview_changed {
                self.trigger_render_preview();
            }
        });

        // Center Viewport Interactive Canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_rect = ui.available_rect_before_wrap();
            let (response, painter) = ui.allocate_painter(available_rect.size(), egui::Sense::click_and_drag());

            // Pan interaction:
            // Middle-click drag, Right-click drag, or Space/Alt + Left-click drag, or Pointer tool
            let is_panning = response.dragged_by(egui::PointerButton::Middle)
                || response.dragged_by(egui::PointerButton::Secondary)
                || (response.dragged_by(egui::PointerButton::Primary)
                    && (ui.input(|i| i.modifiers.alt || i.key_down(egui::Key::Space)) || self.active_tool == WorkstationTool::Pointer));

            if is_panning {
                self.canvas_pan += response.drag_delta();
            }

            // Interactive Tool Handling on Canvas
            if !is_panning
                && self.active_tool != WorkstationTool::Pointer
                && let Some(tex) = &self.texture
            {
                let (tex_w, tex_h) = (tex.size()[0] as f32, tex.size()[1] as f32);
                let img_size = egui::vec2(tex_w * self.canvas_zoom, tex_h * self.canvas_zoom);
                let img_center = available_rect.center() + self.canvas_pan;
                let img_rect = egui::Rect::from_center_size(img_center, img_size);

                if let Some(pointer_pos) = response.interact_pointer_pos().or_else(|| response.hover_pos()) {
                    let rel_x = (pointer_pos.x - img_rect.min.x) / self.canvas_zoom;
                    let rel_y = (pointer_pos.y - img_rect.min.y) / self.canvas_zoom;

                    self.ensure_buffers_dimension(tex_w as u32, tex_h as u32);

                    match self.active_tool {
                        WorkstationTool::BrushMask => {
                            if (response.dragged_by(egui::PointerButton::Primary) || response.clicked_by(egui::PointerButton::Primary))
                                && let Some(mask) = self.mask.as_mut()
                            {
                                mask.paint_brush(rel_x.round() as i32, rel_y.round() as i32, self.brush_radius, self.brush_falloff, 1.0, false);
                                if self.mask_blend_enabled {
                                    self.trigger_render_preview();
                                } else {
                                    self.update_texture(ctx);
                                }
                            }
                        }
                        WorkstationTool::EraserMask => {
                            if (response.dragged_by(egui::PointerButton::Primary) || response.clicked_by(egui::PointerButton::Primary))
                                && let Some(mask) = self.mask.as_mut()
                            {
                                mask.paint_brush(rel_x.round() as i32, rel_y.round() as i32, self.brush_radius, self.brush_falloff, 1.0, true);
                                if self.mask_blend_enabled {
                                    self.trigger_render_preview();
                                } else {
                                    self.update_texture(ctx);
                                }
                            }
                        }
                        WorkstationTool::RectMarquee | WorkstationTool::EllipseMarquee => {
                            if response.drag_started_by(egui::PointerButton::Primary) {
                                self.drag_start_pixel = Some((rel_x.clamp(0.0, tex_w), rel_y.clamp(0.0, tex_h)));
                                self.drag_current_pixel = self.drag_start_pixel;
                            } else if response.dragged_by(egui::PointerButton::Primary) {
                                self.drag_current_pixel = Some((rel_x.clamp(0.0, tex_w), rel_y.clamp(0.0, tex_h)));
                            } else if response.drag_stopped_by(egui::PointerButton::Primary)
                                && let (Some((x0, y0)), Some((x1, y1))) = (self.drag_start_pixel.take(), self.drag_current_pixel.take())
                                && let Some(sel) = self.selection.as_mut()
                            {
                                if self.active_tool == WorkstationTool::RectMarquee {
                                    let min_x = x0.min(x1).round() as u32;
                                    let min_y = y0.min(y1).round() as u32;
                                    let w = (x1 - x0).abs().round().max(1.0) as u32;
                                    let h = (y1 - y0).abs().round().max(1.0) as u32;
                                    sel.select_rect(min_x, min_y, w, h, self.selection_mode);
                                } else {
                                    let cx = (x0 + x1) * 0.5;
                                    let cy = (y0 + y1) * 0.5;
                                    let rx = ((x1 - x0).abs() * 0.5).max(1.0);
                                    let ry = ((y1 - y0).abs() * 0.5).max(1.0);
                                    sel.select_ellipse(cx, cy, rx, ry, self.selection_mode);
                                }
                            }
                        }
                        WorkstationTool::Lasso => {
                            if response.drag_started_by(egui::PointerButton::Primary) {
                                self.lasso_points.clear();
                                if rel_x >= 0.0 && rel_x < tex_w && rel_y >= 0.0 && rel_y < tex_h {
                                    self.lasso_points.push((rel_x, rel_y));
                                }
                            } else if response.dragged_by(egui::PointerButton::Primary) {
                                if rel_x >= 0.0 && rel_x < tex_w && rel_y >= 0.0 && rel_y < tex_h {
                                    let pt = (rel_x, rel_y);
                                    let should_push = match self.lasso_points.last() {
                                        Some(&(lx, ly)) => (lx - rel_x).hypot(ly - rel_y) > 1.0,
                                        None => true,
                                    };
                                    if should_push {
                                        self.lasso_points.push(pt);
                                    }
                                }
                            } else if response.drag_stopped_by(egui::PointerButton::Primary) {
                                if self.lasso_points.len() >= 3
                                    && let Some(sel) = self.selection.as_mut()
                                {
                                    sel.select_polygon(&self.lasso_points, self.selection_mode);
                                }
                                self.lasso_points.clear();
                            }
                        }
                        WorkstationTool::MagicWand => {
                            if response.clicked_by(egui::PointerButton::Primary)
                                && rel_x >= 0.0
                                && rel_x < tex_w
                                && rel_y >= 0.0
                                && rel_y < tex_h
                                && let (Some(sel), Some(src)) = (self.selection.as_mut(), self.source_image.as_ref())
                            {
                                sel.magic_wand(src, rel_x as u32, rel_y as u32, self.wand_tolerance, self.selection_mode);
                            }
                        }
                        WorkstationTool::Pointer => {}
                    }
                }
            }

            // Smooth Zoom interaction centered on mouse cursor
            if response.hovered() {
                let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
                if scroll_delta.abs() > 0.0 {
                    let zoom_factor = if scroll_delta > 0.0 { 1.15 } else { 1.0 / 1.15 };
                    let old_zoom = self.canvas_zoom;
                    let new_zoom = (old_zoom * zoom_factor).clamp(0.05, 32.0);
                    if (new_zoom - old_zoom).abs() > f32::EPSILON {
                        if let Some(pointer_pos) = response.hover_pos() {
                            let canvas_center = available_rect.center();
                            let to_pointer = pointer_pos - canvas_center - self.canvas_pan;
                            self.canvas_pan += to_pointer * (1.0 - new_zoom / old_zoom);
                        }
                        self.canvas_zoom = new_zoom;
                    }
                }
            }

            // Canvas background
            painter.rect_filled(available_rect, 0.0, ui.visuals().extreme_bg_color);

            if let Some(tex) = &self.texture {
                let (tex_w, tex_h) = (tex.size()[0] as f32, tex.size()[1] as f32);
                let img_size = egui::vec2(tex_w * self.canvas_zoom, tex_h * self.canvas_zoom);
                let img_center = available_rect.center() + self.canvas_pan;
                let img_rect = egui::Rect::from_center_size(img_center, img_size);

                let clipped_painter = painter.with_clip_rect(available_rect);

                // Subtle shadow/border around image
                clipped_painter.rect_stroke(img_rect, 0.0, egui::Stroke::new(1.0_f32, Color32::from_gray(50)));

                // Draw image
                clipped_painter.image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);

                // Draw active selection marching ants
                self.marching_ants_phase = (self.marching_ants_phase + 0.4) % 8.0;
                if let Some(ref sel) = self.selection {
                    let segments = sel.extract_marching_ants_segments();
                    if !segments.is_empty() {
                        ctx.request_repaint();
                        let phase = self.marching_ants_phase;
                        for (idx, (p1, p2)) in segments.iter().enumerate() {
                            let start = img_rect.min + egui::vec2(p1.0 * self.canvas_zoom, p1.1 * self.canvas_zoom);
                            let end = img_rect.min + egui::vec2(p2.0 * self.canvas_zoom, p2.1 * self.canvas_zoom);
                            let is_white = (((idx as f32 + phase) as usize) / 4).is_multiple_of(2);
                            let stroke_color = if is_white { Color32::WHITE } else { Color32::BLACK };
                            clipped_painter.line_segment([start, end], egui::Stroke::new(1.5_f32, stroke_color));
                        }
                    }
                }

                // Draw drag marquee preview
                if let (Some((x0, y0)), Some((x1, y1))) = (self.drag_start_pixel, self.drag_current_pixel) {
                    let p0 = img_rect.min + egui::vec2(x0.min(x1) * self.canvas_zoom, y0.min(y1) * self.canvas_zoom);
                    let p1 = img_rect.min + egui::vec2(x0.max(x1) * self.canvas_zoom, y0.max(y1) * self.canvas_zoom);
                    let drag_rect = egui::Rect::from_min_max(p0, p1);
                    let stroke = egui::Stroke::new(1.2_f32, Color32::from_rgb(100, 200, 255));
                    if self.active_tool == WorkstationTool::RectMarquee {
                        clipped_painter.rect_stroke(drag_rect, 0.0, stroke);
                    } else if self.active_tool == WorkstationTool::EllipseMarquee {
                        clipped_painter.circle_stroke(drag_rect.center(), (drag_rect.width() * 0.5).min(drag_rect.height() * 0.5), stroke);
                    }
                }
                if !self.lasso_points.is_empty() {
                    for pair in self.lasso_points.windows(2) {
                        let a = img_rect.min + egui::vec2(pair[0].0 * self.canvas_zoom, pair[0].1 * self.canvas_zoom);
                        let b = img_rect.min + egui::vec2(pair[1].0 * self.canvas_zoom, pair[1].1 * self.canvas_zoom);
                        clipped_painter.line_segment([a, b], egui::Stroke::new(1.2_f32, Color32::from_rgb(100, 200, 255)));
                    }
                }

                // Draw interactive brush cursor outline on hover
                if (self.active_tool == WorkstationTool::BrushMask || self.active_tool == WorkstationTool::EraserMask)
                    && let Some(ptr) = response.hover_pos()
                    && available_rect.contains(ptr)
                {
                    let radius = self.brush_radius * self.canvas_zoom;
                    let brush_color = if self.active_tool == WorkstationTool::BrushMask {
                        Color32::from_rgba_unmultiplied(255, 80, 80, 200)
                    } else {
                        Color32::from_rgba_unmultiplied(100, 180, 255, 200)
                    };
                    clipped_painter.circle_stroke(ptr, radius, egui::Stroke::new(1.2_f32, brush_color));
                }

                // Pixel Grid Overlay (at >= 800% zoom)
                if self.show_pixel_grid && self.canvas_zoom >= 8.0 {
                    let step = self.canvas_zoom;
                    let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 30);
                    let grid_stroke = egui::Stroke::new(1.0_f32, grid_color);

                    let min_x = img_rect.min.x.max(available_rect.min.x);
                    let max_x = img_rect.max.x.min(available_rect.max.x);
                    let min_y = img_rect.min.y.max(available_rect.min.y);
                    let max_y = img_rect.max.y.min(available_rect.max.y);

                    let start_col = ((min_x - img_rect.min.x) / step).floor().max(0.0) as usize;
                    let end_col = ((max_x - img_rect.min.x) / step).ceil().min(tex_w) as usize;
                    for col in start_col..=end_col {
                        let x = img_rect.min.x + col as f32 * step;
                        if x >= available_rect.min.x && x <= available_rect.max.x {
                            clipped_painter.line_segment([egui::pos2(x, min_y), egui::pos2(x, max_y)], grid_stroke);
                        }
                    }

                    let start_row = ((min_y - img_rect.min.y) / step).floor().max(0.0) as usize;
                    let end_row = ((max_y - img_rect.min.y) / step).ceil().min(tex_h) as usize;
                    for row in start_row..=end_row {
                        let y = img_rect.min.y + row as f32 * step;
                        if y >= available_rect.min.y && y <= available_rect.max.y {
                            clipped_painter.line_segment([egui::pos2(min_x, y), egui::pos2(max_x, y)], grid_stroke);
                        }
                    }
                }

                // Retro Aspect Ratio Framing Guide Overlay
                if let Some(target_ar) = self.aspect_guide.aspect_ratio() {
                    let current_ar = img_rect.width() / img_rect.height().max(1.0);
                    let (gw, gh) = if current_ar > target_ar {
                        let h = img_rect.height();
                        let w = h * target_ar;
                        (w, h)
                    } else {
                        let w = img_rect.width();
                        let h = w / target_ar;
                        (w, h)
                    };
                    let guide_rect = egui::Rect::from_center_size(img_rect.center(), egui::vec2(gw, gh));

                    // Translucent letterboxing outside the guide
                    let letterbox_fill = Color32::from_black_alpha(175);
                    if guide_rect.min.y > img_rect.min.y {
                        let top_r = egui::Rect::from_min_max(img_rect.min, egui::pos2(img_rect.max.x, guide_rect.min.y));
                        clipped_painter.rect_filled(top_r, 0.0, letterbox_fill);
                    }
                    if guide_rect.max.y < img_rect.max.y {
                        let bot_r = egui::Rect::from_min_max(egui::pos2(img_rect.min.x, guide_rect.max.y), img_rect.max);
                        clipped_painter.rect_filled(bot_r, 0.0, letterbox_fill);
                    }
                    if guide_rect.min.x > img_rect.min.x {
                        let left_r =
                            egui::Rect::from_min_max(egui::pos2(img_rect.min.x, guide_rect.min.y), egui::pos2(guide_rect.min.x, guide_rect.max.y));
                        clipped_painter.rect_filled(left_r, 0.0, letterbox_fill);
                    }
                    if guide_rect.max.x < img_rect.max.x {
                        let right_r =
                            egui::Rect::from_min_max(egui::pos2(guide_rect.max.x, guide_rect.min.y), egui::pos2(img_rect.max.x, guide_rect.max.y));
                        clipped_painter.rect_filled(right_r, 0.0, letterbox_fill);
                    }

                    // Stroke frame and label
                    let frame_stroke = egui::Stroke::new(1.5_f32, Color32::from_rgb(255, 215, 0));
                    clipped_painter.rect_stroke(guide_rect, 0.0, frame_stroke);
                    clipped_painter.text(
                        guide_rect.min + egui::vec2(6.0, 6.0),
                        egui::Align2::LEFT_TOP,
                        self.aspect_guide.label(),
                        egui::FontId::monospace(10.5),
                        Color32::from_rgb(255, 215, 0),
                    );
                }

                // Floating Viewport HUD
                let hud_size = egui::vec2(580.0, 34.0);
                let hud_rect = egui::Rect::from_min_size(egui::pos2(available_rect.max.x - hud_size.x - 16.0, available_rect.min.y + 16.0), hud_size);

                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(hud_rect), |ui| {
                    egui::Frame::window(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("1:1").on_hover_text("Reset to 100%").clicked() {
                                self.canvas_zoom = 1.0;
                                self.canvas_pan = egui::Vec2::ZERO;
                            }
                            if ui.button("Fit").on_hover_text("Fit to Viewport").clicked() {
                                let margin = 40.0;
                                let scale_x = (available_rect.width() - margin) / tex_w;
                                let scale_y = (available_rect.height() - margin) / tex_h;
                                self.canvas_zoom = scale_x.min(scale_y).clamp(0.05, 32.0);
                                self.canvas_pan = egui::Vec2::ZERO;
                            }
                            if ui.button("−").clicked() {
                                self.canvas_zoom = (self.canvas_zoom / 1.25).clamp(0.05, 32.0);
                            }
                            ui.label(format!("{:.0}%", self.canvas_zoom * 100.0));
                            if ui.button("+").clicked() {
                                self.canvas_zoom = (self.canvas_zoom * 1.25).clamp(0.05, 32.0);
                            }
                            ui.separator();
                            ui.checkbox(&mut self.show_pixel_grid, "Grid");
                            ui.checkbox(&mut self.show_loupe, "Loupe");
                            if self.aspect_guide != RetroAspectGuide::None
                                && ui.small_button("Clear Frame").on_hover_text("Clear aspect ratio guide").clicked()
                            {
                                self.aspect_guide = RetroAspectGuide::None;
                            }
                            ui.separator();
                            if ui.button("⟳").on_hover_text("Rotate 90° Clockwise").clicked() {
                                self.transform_rotate_90_cw();
                            }
                            if ui.button("⇄").on_hover_text("Flip Horizontal").clicked() {
                                self.transform_flip_horizontal();
                            }
                            if ui.button("✂").on_hover_text("Open Crop Workshop").clicked() {
                                self.crop_dialog_open = true;
                            }
                        });
                    });
                });

                // Floating Precision Magnifier Loupe & Subpixel Inspector
                if self.show_loupe
                    && let Some(pointer_pos) = response.hover_pos()
                {
                    let rel_x = (pointer_pos.x - img_rect.min.x) / self.canvas_zoom;
                    let rel_y = (pointer_pos.y - img_rect.min.y) / self.canvas_zoom;
                    if rel_x >= 0.0 && rel_x < tex_w && rel_y >= 0.0 && rel_y < tex_h {
                        let cur_x = (rel_x as usize).min(tex_w as usize - 1);
                        let cur_y = (rel_y as usize).min(tex_h as usize - 1);
                        if let Some(ref img) = self.rendered_image {
                            let center_color = img.get_pixel(cur_x as u32, cur_y as u32).unwrap_or(RgbColor::new(0, 0, 0));
                            let loupe_grid_size = 9;
                            let cell_size = 13.0_f32;
                            let grid_dim = loupe_grid_size as f32 * cell_size;
                            let loupe_w = grid_dim + 20.0;
                            let loupe_h = grid_dim + 56.0;

                            let mut loupe_pos = pointer_pos + egui::vec2(20.0, 20.0);
                            if loupe_pos.x + loupe_w > available_rect.max.x {
                                loupe_pos.x = pointer_pos.x - loupe_w - 20.0;
                            }
                            if loupe_pos.y + loupe_h > available_rect.max.y {
                                loupe_pos.y = pointer_pos.y - loupe_h - 20.0;
                            }

                            let loupe_rect = egui::Rect::from_min_size(loupe_pos, egui::vec2(loupe_w, loupe_h));
                            clipped_painter.rect_filled(loupe_rect, 6.0, Color32::from_black_alpha(235));
                            clipped_painter.rect_stroke(loupe_rect, 6.0, egui::Stroke::new(1.5_f32, Color32::from_gray(180)));

                            let grid_origin = loupe_pos + egui::vec2(10.0, 10.0);
                            let half = (loupe_grid_size / 2) as isize;

                            for gy in -half..=half {
                                for gx in -half..=half {
                                    let px = (cur_x as isize + gx).clamp(0, img.width as isize - 1) as u32;
                                    let py = (cur_y as isize + gy).clamp(0, img.height as isize - 1) as u32;
                                    let c = img.get_pixel(px, py).unwrap_or(RgbColor::new(0, 0, 0));
                                    let cell_color = Color32::from_rgb(c.r, c.g, c.b);
                                    let cell_rect = egui::Rect::from_min_size(
                                        grid_origin + egui::vec2((gx + half) as f32 * cell_size, (gy + half) as f32 * cell_size),
                                        egui::vec2(cell_size, cell_size),
                                    );
                                    clipped_painter.rect_filled(cell_rect, 0.0, cell_color);
                                    clipped_painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(0.5_f32, Color32::from_black_alpha(50)));

                                    if gx == 0 && gy == 0 {
                                        clipped_painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(2.0_f32, Color32::WHITE));
                                    }
                                }
                            }

                            // Subpixel coordinates & color banner
                            let hex_str = format!("#{:02X}{:02X}{:02X}", center_color.r, center_color.g, center_color.b);
                            let coord_str = format!("({}, {})", cur_x, cur_y);
                            let rgb_str = format!("RGB: {}, {}, {}", center_color.r, center_color.g, center_color.b);

                            let info_y = grid_origin.y + grid_dim + 6.0;
                            clipped_painter.text(
                                egui::pos2(loupe_pos.x + 10.0, info_y),
                                egui::Align2::LEFT_TOP,
                                coord_str,
                                egui::FontId::monospace(10.0),
                                Color32::from_gray(220),
                            );
                            clipped_painter.text(
                                egui::pos2(loupe_pos.x + loupe_w - 10.0, info_y),
                                egui::Align2::RIGHT_TOP,
                                hex_str,
                                egui::FontId::monospace(10.0),
                                Color32::YELLOW,
                            );
                            clipped_painter.text(
                                egui::pos2(loupe_pos.x + 10.0, info_y + 13.0),
                                egui::Align2::LEFT_TOP,
                                rgb_str,
                                egui::FontId::monospace(9.5),
                                Color32::LIGHT_GRAY,
                            );
                        }
                    }
                }
            } else {
                painter.text(
                    available_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No image loaded. Use File -> Open Image...",
                    egui::FontId::proportional(16.0),
                    Color32::from_gray(140),
                );
            }
        });

        // Timeline & Viewport Control Toolbar
        egui::TopBottomPanel::bottom("timeline_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Split View controls
                let mut split_changed = false;
                if ui.checkbox(&mut self.split_view_enabled, "Compare View").changed() {
                    split_changed = true;
                }
                if self.split_view_enabled {
                    egui::ComboBox::from_id_salt("split_view_mode_combo").selected_text(self.split_mode.label()).show_ui(ui, |ui| {
                        for m in SplitViewMode::ALL {
                            if ui.selectable_value(&mut self.split_mode, m, m.label()).clicked() {
                                split_changed = true;
                            }
                        }
                    });

                    match self.split_mode {
                        SplitViewMode::Vertical | SplitViewMode::Horizontal => {
                            if ui.add(egui::Slider::new(&mut self.split_position, 0.0..=1.0).text("Position")).changed() {
                                split_changed = true;
                            }
                        }
                        SplitViewMode::Checkerboard => {
                            if ui.add(egui::Slider::new(&mut self.split_checker_tile_size, 8..=128).text("Tile Size")).changed() {
                                split_changed = true;
                            }
                        }
                        SplitViewMode::Difference => {
                            ui.label("Heatmap: Black=0, Blue=Low, Red=High Δ");
                        }
                    }
                }
                if split_changed {
                    self.update_texture(ctx);
                }

                ui.separator();

                // Timeline Transport controls
                let play_label = if self.is_playing { "⏸ Pause" } else { "▶ Play" };
                if ui.button(play_label).clicked() {
                    self.is_playing = !self.is_playing;
                    if self.is_playing {
                        self.last_frame_instant = Instant::now();
                    }
                }

                if ui.button("⏮").clicked() {
                    let prev = self.timeline_frame.saturating_sub(1);
                    self.set_current_frame(prev);
                }

                if ui.button("⏭").clicked() {
                    let next = if self.timeline_frame + 1 < self.timeline_max_frames {
                        self.timeline_frame + 1
                    } else if self.loop_mode {
                        0
                    } else {
                        self.timeline_frame
                    };
                    self.set_current_frame(next);
                }

                let max_slider = self.timeline_max_frames.saturating_sub(1);
                if ui.add(egui::Slider::new(&mut self.timeline_frame, 0..=max_slider).text("Frame")).changed() {
                    self.set_current_frame(self.timeline_frame);
                }

                ui.add(egui::Slider::new(&mut self.target_fps, 1..=60).text("FPS"));
                ui.checkbox(&mut self.loop_mode, "Loop");

                let mut cadence_changed = false;
                egui::ComboBox::from_label("Cadence")
                    .selected_text(match self.cadence_mode {
                        CadenceMode::Passthrough => "Passthrough",
                        CadenceMode::Cinematic24 => "Cinematic 24 (3:2)",
                        CadenceMode::AnimeOnTwos12 => "Anime 12 (On-Twos)",
                        CadenceMode::RetroCrunch8 => "Retro 8 FPS",
                        CadenceMode::StopMotion4 => "Stop-Motion 4 FPS",
                        CadenceMode::InterlacedFields => "Interlaced Fields",
                    })
                    .show_ui(ui, |ui| {
                        cadence_changed |= ui.selectable_value(&mut self.cadence_mode, CadenceMode::Passthrough, "Passthrough").changed();
                        cadence_changed |= ui.selectable_value(&mut self.cadence_mode, CadenceMode::Cinematic24, "Cinematic 24 (3:2)").changed();
                        cadence_changed |= ui.selectable_value(&mut self.cadence_mode, CadenceMode::AnimeOnTwos12, "Anime 12 (On-Twos)").changed();
                        cadence_changed |= ui.selectable_value(&mut self.cadence_mode, CadenceMode::RetroCrunch8, "Retro 8 FPS").changed();
                        cadence_changed |= ui.selectable_value(&mut self.cadence_mode, CadenceMode::StopMotion4, "Stop-Motion 4 FPS").changed();
                        cadence_changed |= ui.selectable_value(&mut self.cadence_mode, CadenceMode::InterlacedFields, "Interlaced Fields").changed();
                    });

                let mut onion_changed = false;
                onion_changed |= ui.checkbox(&mut self.onion_skin_enabled, "Onion Skin").changed();
                if self.onion_skin_enabled {
                    onion_changed |= ui.add(egui::Slider::new(&mut self.onion_skin_config.opacity, 0.05..=0.9).text("Ghost α")).changed();
                    onion_changed |= ui.add(egui::Slider::new(&mut self.onion_skin_config.prev_frames, 1..=4).text("Prev")).changed();
                    onion_changed |= ui.add(egui::Slider::new(&mut self.onion_skin_config.next_frames, 0..=4).text("Next")).changed();
                }

                if cadence_changed || onion_changed {
                    self.set_current_frame(self.timeline_frame);
                }

                ui.separator();
                let mut tc_changed = false;
                tc_changed |= ui
                    .checkbox(&mut self.temporal_coherence_enabled, "Temporal Coherence")
                    .on_hover_text("Suppress inter-frame high-frequency dither flicker")
                    .changed();
                if self.temporal_coherence_enabled {
                    tc_changed |= ui.add(egui::Slider::new(&mut self.temporal_coherence_damping, 0.1..=0.95).text("Damping")).changed();
                    tc_changed |= ui.add(egui::Slider::new(&mut self.temporal_coherence_threshold, 0.01..=0.20).text("Motion Thresh")).changed();
                }
                if tc_changed {
                    self.trigger_render_preview();
                }

                ui.separator();
                let audio_loaded = self.audio_track.is_some();
                let wav_btn_text = if audio_loaded { "🎵 WAV Loaded" } else { "🎵 Load WAV..." };
                if ui.button(wav_btn_text).clicked()
                    && let Some(path) = rfd::FileDialog::new().add_filter("WAV Audio", &["wav"]).pick_file()
                    && let Ok(bytes) = std::fs::read(&path)
                    && let Ok(track) = AudioTrack::parse_wav(&bytes)
                {
                    let dur = if track.sample_rate > 0 && track.channels > 0 {
                        (track.samples.len() as f64) / (track.sample_rate as f64 * track.channels as f64)
                    } else {
                        0.0
                    };
                    self.status_text = format!("Loaded WAV audio track: {} Hz, {} ch, {:.2}s", track.sample_rate, track.channels, dur);
                    self.audio_track = Some(track);
                    self.audio_reactive_enabled = true;
                    self.trigger_render();
                }
                if audio_loaded {
                    if ui.checkbox(&mut self.audio_reactive_enabled, "Reactive").changed() {
                        self.trigger_render();
                    }
                    if self.audio_reactive_enabled {
                        ui.colored_label(Color32::from_rgb(255, 100, 100), format!("Bass: {:.2}", self.audio_spectrum.bass));
                        ui.colored_label(Color32::from_rgb(100, 255, 100), format!("Mid: {:.2}", self.audio_spectrum.mid));
                        ui.colored_label(Color32::from_rgb(100, 200, 255), format!("Treble: {:.2}", self.audio_spectrum.treble));
                    }
                }
            });
        });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_text);
                ui.separator();
                ui.label("Snapshots:");
                for (idx, label) in ['A', 'B', 'C', 'D'].iter().enumerate() {
                    let is_stored = self.snapshots[idx].is_some();
                    let btn_text = if is_stored { format!("[{label}]") } else { format!(" {label} ") };
                    let btn = ui.small_button(btn_text).on_hover_text(if let Some(ref s) = self.snapshots[idx] {
                        format!(
                            "Slot {label}: {} ({})\nClick to recall (F{})\nShift+Click or Right-Click to save current (Shift+F{})",
                            s.algo,
                            s.palette_name,
                            idx + 5,
                            idx + 5
                        )
                    } else {
                        format!("Slot {label}: [Empty]\nClick or Right-Click to save current (Shift+F{})", idx + 5)
                    });
                    if btn.clicked() {
                        if ctx.input(|i| i.modifiers.shift) || !is_stored {
                            let snap = self.capture_preset(format!("Snapshot {label}"));
                            self.status_text = format!("Stored Snapshot {label}: {} ({})", snap.algo, snap.palette_name);
                            self.snapshots[idx] = Some(snap);
                        } else if let Some(snap) = self.snapshots[idx].clone() {
                            self.record_undo_state();
                            self.apply_preset(&snap);
                            self.status_text = format!("Recalled Snapshot {label}: {} ({})", snap.algo, snap.palette_name);
                            self.trigger_render();
                        }
                    } else if btn.secondary_clicked() {
                        let snap = self.capture_preset(format!("Snapshot {label}"));
                        self.status_text = format!("Stored Snapshot {label}: {} ({})", snap.algo, snap.palette_name);
                        self.snapshots[idx] = Some(snap);
                    }
                }
                ui.separator();
                ui.label(format!("Latency: {:.2} ms", self.render_latency_ms)).on_hover_text(format!(
                    "Pipeline Telemetry:\n• Dither: {:.2} ms\n• Effects: {:.2} ms\n• Histogram: {:.2} ms\n• Total: {:.2} ms",
                    self.telemetry_dither_ms, self.telemetry_effects_ms, self.telemetry_histogram_ms, self.render_latency_ms
                ));
                if self.is_rendering {
                    ui.spinner();
                    ui.label("Rendering on Rayon worker thread...");
                } else {
                    ui.label("Engine Idle");
                }
            });
        });

        // Playback & Animation Loop
        if self.is_playing {
            let now = Instant::now();
            let frame_duration = 1.0 / (self.target_fps as f64).max(1.0);
            if now.duration_since(self.last_frame_instant).as_secs_f64() >= frame_duration {
                self.last_frame_instant = now;
                let next = if self.timeline_frame + 1 < self.timeline_max_frames {
                    self.timeline_frame + 1
                } else if self.loop_mode {
                    0
                } else {
                    self.is_playing = false;
                    self.timeline_frame
                };
                self.set_current_frame(next);
            }
            ctx.request_repaint();
        } else if self.temporal_noise_enabled {
            self.frame_counter = self.frame_counter.wrapping_add(1);
            self.trigger_render();
            ctx.request_repaint();
        }
    }
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_title("DitherCraft — Retro Graphics Workstation");

    const ICON_BYTES: &[u8] = include_bytes!("../../../assets/app-icon/hicolor/256x256/apps/io.github.dithercraft.png");
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_BYTES) {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let native_options = eframe::NativeOptions { viewport, ..Default::default() };

    eframe::run_native("DitherCraft", native_options, Box::new(|cc| Ok(Box::new(DitherCraftApp::new(cc)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retro_aspect_guide_ratios_and_labels() {
        for guide in RetroAspectGuide::ALL {
            assert!(!guide.label().is_empty());
        }
        assert_eq!(RetroAspectGuide::None.aspect_ratio(), None);
        assert!((RetroAspectGuide::GameBoy.aspect_ratio().unwrap() - 10.0 / 9.0).abs() < 1e-4);
        assert!((RetroAspectGuide::NesSnes.aspect_ratio().unwrap() - 8.0 / 7.0).abs() < 1e-4);
        assert!((RetroAspectGuide::StandardCrt.aspect_ratio().unwrap() - 4.0 / 3.0).abs() < 1e-4);
        assert!((RetroAspectGuide::ArcadeTate.aspect_ratio().unwrap() - 3.0 / 4.0).abs() < 1e-4);
        assert!((RetroAspectGuide::Square.aspect_ratio().unwrap() - 1.0).abs() < 1e-4);
        assert!((RetroAspectGuide::Widescreen.aspect_ratio().unwrap() - 16.0 / 9.0).abs() < 1e-4);
    }

    #[test]
    fn test_studio_preset_serde_and_bundled_presets() {
        let presets = StudioPreset::bundled_presets();
        assert!(!presets.is_empty());
        for p in &presets {
            let json = serde_json::to_string(p).expect("serialize preset");
            let de: StudioPreset = serde_json::from_str(&json).expect("deserialize preset");
            assert_eq!(de.name, p.name);
            assert_eq!(de.levels_black, p.levels_black);
            assert_eq!(de.levels_white, p.levels_white);
            assert_eq!(de.cvd_mode, p.cvd_mode);
        }
        assert_eq!(default_levels_black(), 0);
        assert_eq!(default_levels_white(), 255);
        assert!((default_levels_gamma() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_workstation_tool_labels_and_defaults() {
        assert_eq!(WorkstationTool::default(), WorkstationTool::Pointer);
        for tool in WorkstationTool::ALL {
            assert!(!tool.label().is_empty());
        }
    }

    #[test]
    fn test_mask_and_selection_integration() {
        let mut mask = MaskBuffer::new(64, 64).expect("create mask");
        assert!(mask.is_empty());
        mask.paint_brush(32, 32, 10.0, 0.5, 1.0, false);
        assert!(!mask.is_empty());

        let sel = SelectionBuffer::from_mask(mask.clone());
        assert!(!sel.is_empty());
        let roundtrip_mask = sel.to_mask();
        assert_eq!(roundtrip_mask.width, 64);
        assert_eq!(roundtrip_mask.height, 64);
    }
}
