#![allow(clippy::chunks_exact_to_as_chunks)]

use dither_core::{
    AudioSpectrum, AudioTrack, ImageBuffer, Mp4Demuxer,
    algorithms::*,
    color::RgbColor,
    effects::*,
    palette::{ColorMetric, Palette, Swatch},
    temporal::TemporalNoisePattern,
    vector::{BlackVectorIsolation, GCodeConfig, GCodeExporter, MultiColorSvgSeparation, export_smooth_bezier_svg},
};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::{AnimationDecoder, Frame, RgbaImage};
use serde::{Deserialize, Serialize};
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

/// Workstation snapshot capturing complete algorithm, palette, and effects configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StudioPreset {
    pub name: String,
    pub algo: String,
    pub palette_name: String,
    pub metric: ColorMetric,
    pub tonal_cycle_step: usize,

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

            temporal_enabled: self.temporal_noise_enabled,
            temporal_pattern: self.temporal_pattern,
            temporal_intensity: self.temporal_intensity,
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

    fn set_current_frame(&mut self, frame: u32) {
        self.timeline_frame = frame;
        self.frame_counter = frame;
        if !self.source_frames.is_empty() {
            let idx = (frame as usize).min(self.source_frames.len() - 1);
            self.source_image = Some(self.source_frames[idx].clone());
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

        let glow_opt = if self.glow_enabled {
            Some(EpsilonGlow { threshold: self.glow_threshold, radius: self.glow_radius, intensity: self.glow_intensity })
        } else {
            None
        };

        let chromatic_opt = if self.chromatic_enabled {
            Some(ChromaticAberration { shift_x: chromatic_shift, shift_y: 0, radial: self.chromatic_radial })
        } else {
            None
        };

        let glitch_opt = if self.glitch_enabled {
            Some(JpegGlitchCore { mode: self.glitch_mode, intensity: glitch_intensity, block_size: 8, seed: 42 })
        } else {
            None
        };

        let crt_opt = if self.crt_enabled {
            Some(VintageCrtSimulation { scanline_intensity: crt_scanline, barrel_curvature: self.crt_barrel, vignette_intensity: self.crt_vignette })
        } else {
            None
        };

        let blur_opt = if self.blur_enabled { Some(GaussianBlur::new(self.blur_sigma)) } else { None };

        let sharpen_opt = if self.sharpen_enabled { Some(Sharpen::new(self.sharpen_amount, 1.0)) } else { None };

        let temporal_opt =
            if self.temporal_noise_enabled { Some((self.temporal_pattern, self.frame_counter, self.temporal_intensity)) } else { None };

        self.is_rendering = true;
        let gen_id = self.render_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let cancel_token = self.render_generation.clone();
        let ctx = self.egui_ctx.clone();

        rayon::spawn(move || {
            let start = Instant::now();
            if cancel_token.load(Ordering::Relaxed) != gen_id {
                return;
            }

            let algo: Box<dyn DitherAlgorithm> = create_algorithm(&algo_name).unwrap_or_else(|| Box::new(FloydSteinberg::default()));

            let mut out = algo.apply(&source, &palette).unwrap_or_else(|_| (*source).clone());

            if cancel_token.load(Ordering::Relaxed) != gen_id {
                return;
            }

            // Post-effects stack
            if let Some(glow) = glow_opt {
                out = glow.apply(&out).unwrap_or(out);
            }
            if cancel_token.load(Ordering::Relaxed) != gen_id {
                return;
            }

            if let Some(chromatic) = chromatic_opt {
                out = chromatic.apply(&out).unwrap_or(out);
            }
            if let Some(glitch) = glitch_opt {
                out = glitch.apply(&out).unwrap_or(out);
            }
            if let Some(crt) = crt_opt {
                out = crt.apply(&out).unwrap_or(out);
            }
            if let Some(blur) = blur_opt {
                out = blur.apply(&out).unwrap_or(out);
            }
            if let Some(sharpen) = sharpen_opt {
                out = sharpen.apply(&out).unwrap_or(out);
            }
            if let Some((pattern, frame, intensity)) = temporal_opt {
                out = pattern.apply(&out, frame, intensity).unwrap_or(out);
            }

            if cancel_token.load(Ordering::Relaxed) != gen_id {
                return;
            }

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

            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            let _ = tx.send(RenderJobResult { buffer: out, duration_ms, gen_id, swatch_counts });
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
                self.rendered_image = Some(result.buffer);
                self.render_latency_ms = result.duration_ms;
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

        // Top Menu Bar
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Image, GIF, or Video...").clicked() {
                        let path =
                            rfd::FileDialog::new().add_filter("Media Files", &["png", "jpg", "jpeg", "webp", "bmp", "gif", "mp4", "m4v"]).pick_file();
                        if let Some(path) = path {
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            let is_gif = ext.eq_ignore_ascii_case("gif");
                            let is_mp4 = ext.eq_ignore_ascii_case("mp4") || ext.eq_ignore_ascii_case("m4v");

                            if is_mp4 && let Ok(bytes) = std::fs::read(&path) {
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
                                        self.status_text =
                                            format!("Loaded MP4 video ({} frames, {:.1} FPS)", self.source_frames.len(), track.nominal_fps());
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
                            } else if is_gif
                                && let Ok(file) = std::fs::File::open(&path)
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
                            } else if let Ok(reader) = image::ImageReader::open(&path)
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
                                    self.trigger_render();
                                }
                            }
                        }
                        ui.close_menu();
                    }

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
                        if let Some(ref img) = self.rendered_image
                            && let Some(path) = rfd::FileDialog::new().add_filter("PNG Image", &["png"]).save_file()
                        {
                            let _ = image::save_buffer(&path, &img.data, img.width, img.height, image::ExtendedColorType::Rgb8);
                        }
                        ui.close_menu();
                    }

                    if ui.button("Export Animated GIF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("GIF Animation", &["gif"]).save_file() {
                            let num_frames = if self.source_frames.len() > 1 { self.source_frames.len() } else { self.timeline_max_frames as usize };
                            let frames_to_render: Vec<Arc<ImageBuffer>> = if self.source_frames.len() > 1 {
                                self.source_frames.clone()
                            } else if let Some(ref src) = self.source_image {
                                vec![src.clone(); num_frames]
                            } else {
                                Vec::new()
                            };

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
                                let rendered: Vec<_> = frames_to_render
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
                        if let Some(path) = rfd::FileDialog::new().add_filter("APNG Animation", &["png", "apng"]).save_file() {
                            let num_frames = if self.source_frames.len() > 1 { self.source_frames.len() } else { self.timeline_max_frames as usize };
                            let frames_to_render: Vec<Arc<ImageBuffer>> = if self.source_frames.len() > 1 {
                                self.source_frames.clone()
                            } else if let Some(ref src) = self.source_image {
                                vec![src.clone(); num_frames]
                            } else {
                                Vec::new()
                            };

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
                                let rendered: Vec<_> = frames_to_render
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

                    if ui.button("Export Black Vector SVG...").clicked() {
                        let save_target = self
                            .rendered_image
                            .as_ref()
                            .and_then(|img| rfd::FileDialog::new().add_filter("SVG Vector", &["svg"]).save_file().map(|path| (img, path)));
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
                        let save_target = self
                            .rendered_image
                            .as_ref()
                            .and_then(|img| rfd::FileDialog::new().add_filter("Layered SVG", &["svg"]).save_file().map(|path| (img, path)));
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
                        let save_target = self
                            .rendered_image
                            .as_ref()
                            .and_then(|img| rfd::FileDialog::new().add_filter("CNC G-Code", &["gcode", "nc"]).save_file().map(|path| (img, path)));
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
                        let save_target = self
                            .rendered_image
                            .as_ref()
                            .and_then(|img| rfd::FileDialog::new().add_filter("Smooth Bézier SVG", &["svg"]).save_file().map(|path| (img, path)));
                        if let Some((img, path)) = save_target
                            && let Ok(svg) = export_smooth_bezier_svg(img, 128, 0.5)
                        {
                            let _ = std::fs::write(path, svg);
                            self.status_text = "Exported smooth Bézier spline SVG".into();
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

                ui.menu_button("Presets", |ui| {
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

                ui.menu_button("Help", |ui| {
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
                    for info in filtered {
                        if ui.selectable_value(&mut self.selected_algo, info.name.into(), info.name).clicked() {
                            changed = true;
                        }
                    }
                });
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

            // Stackable Downstream Effects
            ui.group(|ui| {
                ui.label("Downstream Effects Stack:");
                if ui.checkbox(&mut self.glow_enabled, "Epsilon Glow Bloom").changed() {
                    changed = true;
                }
                if self.glow_enabled {
                    check_slider(ui, egui::Slider::new(&mut self.glow_threshold, 50..=250).text("Threshold"), &mut changed, &mut preview_changed);
                    check_slider(ui, egui::Slider::new(&mut self.glow_radius, 1..=12).text("Radius"), &mut changed, &mut preview_changed);
                }

                if ui.checkbox(&mut self.chromatic_enabled, "Chromatic Aberration").changed() {
                    changed = true;
                }
                if self.chromatic_enabled {
                    check_slider(ui, egui::Slider::new(&mut self.chromatic_shift, -15..=15).text("Shift X"), &mut changed, &mut preview_changed);
                    if ui.checkbox(&mut self.chromatic_radial, "Radial Shift").changed() {
                        changed = true;
                    }
                }

                if ui.checkbox(&mut self.glitch_enabled, "JPEG Glitch Core").changed() {
                    changed = true;
                }
                if self.glitch_enabled {
                    egui::ComboBox::from_id_salt("glitch_mode_picker")
                        .selected_text(match self.glitch_mode {
                            JpegSubMode::BlockShift => "Block Shift",
                            JpegSubMode::ChannelSwap => "Channel Swap",
                            JpegSubMode::ScanLineOffset => "Scan Line Offset",
                            JpegSubMode::BlockScramble => "Block Scramble",
                            JpegSubMode::InterlaceCorruption => "Interlace Corruption",
                        })
                        .show_ui(ui, |ui| {
                            for (mode, label) in [
                                (JpegSubMode::BlockShift, "Block Shift"),
                                (JpegSubMode::ChannelSwap, "Channel Swap"),
                                (JpegSubMode::ScanLineOffset, "Scan Line Offset"),
                                (JpegSubMode::BlockScramble, "Block Scramble"),
                                (JpegSubMode::InterlaceCorruption, "Interlace Corruption"),
                            ] {
                                if ui.selectable_value(&mut self.glitch_mode, mode, label).clicked() {
                                    changed = true;
                                }
                            }
                        });
                    check_slider(
                        ui,
                        egui::Slider::new(&mut self.glitch_intensity, 0.05..=1.0).text("Glitch Intensity"),
                        &mut changed,
                        &mut preview_changed,
                    );
                }

                if ui.checkbox(&mut self.crt_enabled, "Vintage CRT Simulation").changed() {
                    changed = true;
                }
                if self.crt_enabled {
                    check_slider(ui, egui::Slider::new(&mut self.crt_scanline, 0.0..=1.0).text("Scanlines"), &mut changed, &mut preview_changed);
                    check_slider(ui, egui::Slider::new(&mut self.crt_barrel, 0.0..=0.3).text("Barrel"), &mut changed, &mut preview_changed);
                    check_slider(ui, egui::Slider::new(&mut self.crt_vignette, 0.0..=1.0).text("Vignette"), &mut changed, &mut preview_changed);
                }

                if ui.checkbox(&mut self.blur_enabled, "Gaussian Blur").changed() {
                    changed = true;
                }
                if self.blur_enabled {
                    check_slider(ui, egui::Slider::new(&mut self.blur_sigma, 0.2..=5.0).text("Sigma"), &mut changed, &mut preview_changed);
                }

                if ui.checkbox(&mut self.sharpen_enabled, "Sharpen").changed() {
                    changed = true;
                }
                if self.sharpen_enabled {
                    check_slider(ui, egui::Slider::new(&mut self.sharpen_amount, 0.1..=3.0).text("Amount"), &mut changed, &mut preview_changed);
                }
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
            // Middle-click drag, Right-click drag, or Space/Alt + Left-click drag
            let is_panning = response.dragged_by(egui::PointerButton::Middle)
                || response.dragged_by(egui::PointerButton::Secondary)
                || (response.dragged_by(egui::PointerButton::Primary) && ui.input(|i| i.modifiers.alt || i.key_down(egui::Key::Space)));

            if is_panning {
                self.canvas_pan += response.drag_delta();
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

                // Floating Viewport HUD
                let hud_size = egui::vec2(280.0, 34.0);
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
                        });
                    });
                });
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
                ui.label(format!("Latency: {:.2} ms", self.render_latency_ms));
                ui.separator();
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
