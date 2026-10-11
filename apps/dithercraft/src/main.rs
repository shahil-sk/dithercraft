#![allow(clippy::chunks_exact_to_as_chunks)]

use dither_core::{
    ImageBuffer,
    algorithms::*,
    color::RgbColor,
    effects::*,
    palette::{ColorMetric, Palette},
    temporal::TemporalNoisePattern,
    vector::BlackVectorIsolation,
};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use std::sync::Arc;
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

struct RenderJobResult {
    buffer: ImageBuffer,
    duration_ms: f64,
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
    split_position: f32,

    // Timeline Editor & Transport
    is_playing: bool,
    target_fps: u32,
    loop_mode: bool,
    timeline_frame: u32,
    timeline_max_frames: u32,
    last_frame_instant: Instant,

    // Threading channels
    tx: Sender<RenderJobResult>,
    rx: Receiver<RenderJobResult>,
    is_rendering: bool,
}

impl DitherCraftApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();

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

        Self {
            source_image: Some(Arc::new(default_img)),
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
            split_position: 0.5,

            is_playing: false,
            target_fps: 30,
            loop_mode: true,
            timeline_frame: 0,
            timeline_max_frames: 60,
            last_frame_instant: Instant::now(),

            tx,
            rx,
            is_rendering: false,
        }
    }

    fn trigger_render(&mut self) {
        let Some(source) = self.source_image.clone() else { return };
        let tx = self.tx.clone();
        let algo_name = self.selected_algo.clone();
        let palette = self.active_palette.cycle_tonal(self.tonal_cycle_step);

        let glow_opt = if self.glow_enabled {
            Some(EpsilonGlow { threshold: self.glow_threshold, radius: self.glow_radius, intensity: self.glow_intensity })
        } else {
            None
        };

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

        let temporal_opt =
            if self.temporal_noise_enabled { Some((self.temporal_pattern, self.frame_counter, self.temporal_intensity)) } else { None };

        self.is_rendering = true;

        rayon::spawn(move || {
            let start = Instant::now();
            let algo: Box<dyn DitherAlgorithm> = create_algorithm(&algo_name).unwrap_or_else(|| Box::new(FloydSteinberg::default()));

            let mut out = algo.apply(&source, &palette).unwrap_or_else(|_| (*source).clone());

            // Post-effects stack
            if let Some(glow) = glow_opt {
                out = glow.apply(&out).unwrap_or(out);
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

            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            let _ = tx.send(RenderJobResult { buffer: out, duration_ms });
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
                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) * 3;
                        if x == split_x {
                            color_data.push(255);
                            color_data.push(255);
                            color_data.push(255);
                            color_data.push(255);
                        } else if x < split_x {
                            color_data.push(src.data[idx]);
                            color_data.push(src.data[idx + 1]);
                            color_data.push(src.data[idx + 2]);
                            color_data.push(255);
                        } else {
                            color_data.push(res.data[idx]);
                            color_data.push(res.data[idx + 1]);
                            color_data.push(res.data[idx + 2]);
                            color_data.push(255);
                        }
                    }
                }
            } else {
                for chunk in res.data.chunks_exact(3) {
                    color_data.push(chunk[0]);
                    color_data.push(chunk[1]);
                    color_data.push(chunk[2]);
                    color_data.push(255);
                }
            }

            let color_image = ColorImage::from_rgba_unmultiplied([width, height], &color_data);
            self.texture = Some(ctx.load_texture("rendered_preview", color_image, TextureOptions::NEAREST));
        }
    }
}

impl eframe::App for DitherCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll worker thread results
        while let Ok(result) = self.rx.try_recv() {
            self.rendered_image = Some(result.buffer);
            self.render_latency_ms = result.duration_ms;
            self.is_rendering = false;
            self.update_texture(ctx);
        }

        // Apply theme
        self.theme.apply(ctx);

        // Top Menu Bar
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Image...").clicked() {
                        let path = rfd::FileDialog::new().add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp"]).pick_file();
                        if let Some(path) = path
                            && let Ok(reader) = image::ImageReader::open(&path)
                            && let Ok(dyn_img) = reader.decode()
                        {
                            let rgb = dyn_img.to_rgb8();
                            let (w, h) = rgb.dimensions();
                            if let Ok(buf) = ImageBuffer::from_raw(w, h, rgb.into_raw()) {
                                self.source_image = Some(Arc::new(buf));
                                self.trigger_render();
                            }
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

                    if ui.button("Export Black Vector SVG...").clicked() {
                        let save_target = self
                            .rendered_image
                            .as_ref()
                            .and_then(|img| rfd::FileDialog::new().add_filter("SVG Vector", &["svg"]).save_file().map(|path| (img, path)));
                        if let Some((img, path)) = save_target {
                            let isolator = BlackVectorIsolation::default();
                            if let Ok(svg) = isolator.trace_to_svg(img) {
                                let _ = std::fs::write(path, svg);
                            }
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Import Palette...").clicked() {
                        if let Some(path) = rfd::FileDialog::new().add_filter("Palette", &["json", "hex", "gpl", "pal"]).pick_file()
                            && let Ok(pal) = Palette::import_from_file(&path)
                        {
                            self.selected_palette_name = pal.name.clone();
                            self.active_palette = pal;
                            self.trigger_render();
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
            });
        });

        // Left Control Panel
        egui::SidePanel::left("control_panel").min_width(280.0).show(ctx, |ui| {
            ui.heading("DitherCraft Studio");
            ui.separator();

            let mut changed = false;

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
                    })
                    .show_ui(ui, |ui| {
                        for (metric, label) in [
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
                    if ui.button("Sort Luminance").clicked() {
                        self.active_palette.sort_by_luminance(true);
                        changed = true;
                    }
                    if ui.button("Randomize Unlocked").clicked() {
                        self.active_palette.randomize_unlocked(Instant::now().elapsed().as_nanos() as u64);
                        changed = true;
                    }
                });

                // Swatches list with lock toggles
                ui.label("Swatches & Slot Locks:");
                let mut lock_changed = false;
                for swatch in &mut self.active_palette.swatches {
                    ui.horizontal(|ui| {
                        let (r, g, b) = (swatch.color.r, swatch.color.g, swatch.color.b);
                        let c = Color32::from_rgb(r, g, b);
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, c);
                        if ui.checkbox(&mut swatch.locked, format!("#{r:02X}{g:02X}{b:02X}")).changed() {
                            lock_changed = true;
                        }
                    });
                }
                if lock_changed {
                    changed = true;
                }
            });

            // Tonal Cycling
            ui.group(|ui| {
                ui.label("Tonal Cycling:");
                if ui.add(egui::Slider::new(&mut self.tonal_cycle_step, 0..=8).text("Cycle Step")).changed() {
                    changed = true;
                }
            });

            // Stackable Downstream Effects
            ui.group(|ui| {
                ui.label("Downstream Effects Stack:");
                if ui.checkbox(&mut self.glow_enabled, "Epsilon Glow Bloom").changed() {
                    changed = true;
                }
                if self.glow_enabled {
                    if ui.add(egui::Slider::new(&mut self.glow_threshold, 50..=250).text("Threshold")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::Slider::new(&mut self.glow_radius, 1..=12).text("Radius")).changed() {
                        changed = true;
                    }
                }

                if ui.checkbox(&mut self.chromatic_enabled, "Chromatic Aberration").changed() {
                    changed = true;
                }
                if self.chromatic_enabled {
                    if ui.add(egui::Slider::new(&mut self.chromatic_shift, -15..=15).text("Shift X")).changed() {
                        changed = true;
                    }
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
                    if ui.add(egui::Slider::new(&mut self.glitch_intensity, 0.05..=1.0).text("Glitch Intensity")).changed() {
                        changed = true;
                    }
                }

                if ui.checkbox(&mut self.crt_enabled, "Vintage CRT Simulation").changed() {
                    changed = true;
                }
                if self.crt_enabled {
                    if ui.add(egui::Slider::new(&mut self.crt_scanline, 0.0..=1.0).text("Scanlines")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::Slider::new(&mut self.crt_barrel, 0.0..=0.3).text("Barrel")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::Slider::new(&mut self.crt_vignette, 0.0..=1.0).text("Vignette")).changed() {
                        changed = true;
                    }
                }

                if ui.checkbox(&mut self.blur_enabled, "Gaussian Blur").changed() {
                    changed = true;
                }
                if self.blur_enabled && ui.add(egui::Slider::new(&mut self.blur_sigma, 0.2..=5.0).text("Sigma")).changed() {
                    changed = true;
                }

                if ui.checkbox(&mut self.sharpen_enabled, "Sharpen").changed() {
                    changed = true;
                }
                if self.sharpen_enabled && ui.add(egui::Slider::new(&mut self.sharpen_amount, 0.1..=3.0).text("Amount")).changed() {
                    changed = true;
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

                    if ui.add(egui::Slider::new(&mut self.temporal_intensity, 0.05..=1.0).text("Noise Intensity")).changed() {
                        changed = true;
                    }
                }
            });

            // Debounce setting
            ui.checkbox(&mut self.debounce_mode, "Debounce Rendering");

            if changed {
                self.trigger_render();
            }
        });

        // Center Viewport
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                ui.centered_and_justified(|ui| {
                    ui.image(tex);
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No image loaded. Use File -> Open Image...");
                });
            }
        });

        // Timeline & Viewport Control Toolbar
        egui::TopBottomPanel::bottom("timeline_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Split View controls
                let mut split_changed = false;
                if ui.checkbox(&mut self.split_view_enabled, "Split View (Before / After)").changed() {
                    split_changed = true;
                }
                if self.split_view_enabled && ui.add(egui::Slider::new(&mut self.split_position, 0.0..=1.0).text("Split")).changed() {
                    split_changed = true;
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
                    self.timeline_frame = self.timeline_frame.saturating_sub(1);
                    self.frame_counter = self.timeline_frame;
                    self.trigger_render();
                }

                if ui.button("⏭").clicked() {
                    if self.timeline_frame + 1 < self.timeline_max_frames {
                        self.timeline_frame += 1;
                    } else if self.loop_mode {
                        self.timeline_frame = 0;
                    }
                    self.frame_counter = self.timeline_frame;
                    self.trigger_render();
                }

                if ui.add(egui::Slider::new(&mut self.timeline_frame, 0..=self.timeline_max_frames).text("Frame")).changed() {
                    self.frame_counter = self.timeline_frame;
                    self.trigger_render();
                }

                ui.add(egui::Slider::new(&mut self.target_fps, 1..=60).text("FPS"));
                ui.checkbox(&mut self.loop_mode, "Loop");
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
                if self.timeline_frame + 1 < self.timeline_max_frames {
                    self.timeline_frame += 1;
                } else if self.loop_mode {
                    self.timeline_frame = 0;
                } else {
                    self.is_playing = false;
                }
                self.frame_counter = self.timeline_frame;
                self.trigger_render();
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
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_title("DitherCraft — Retro Graphics Workstation"),
        ..Default::default()
    };

    eframe::run_native("DitherCraft", native_options, Box::new(|cc| Ok(Box::new(DitherCraftApp::new(cc)))))
}
