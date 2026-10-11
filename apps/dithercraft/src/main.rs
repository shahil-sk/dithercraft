#![allow(clippy::chunks_exact_to_as_chunks)]

use dither_core::{
    ImageBuffer, algorithms::*, color::RgbColor, effects::*, palette::Palette, renderers::*, temporal::TemporalNoisePattern,
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

    temporal_noise_enabled: bool,
    temporal_pattern: TemporalNoisePattern,
    temporal_intensity: f32,
    frame_counter: u32,

    // UI & Viewport
    theme: StudioTheme,
    debounce_mode: bool,
    status_text: String,
    render_latency_ms: f64,
    _split_slider: f32,

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

            temporal_noise_enabled: false,
            temporal_pattern: TemporalNoisePattern::CrtScanBeamJitter,
            temporal_intensity: 0.5,
            frame_counter: 0,

            theme: StudioTheme::StudioDark,
            debounce_mode: false,
            status_text: "Ready".into(),
            render_latency_ms: 0.0,
            _split_slider: 1.0,

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

        let temporal_opt =
            if self.temporal_noise_enabled { Some((self.temporal_pattern, self.frame_counter, self.temporal_intensity)) } else { None };

        self.is_rendering = true;

        rayon::spawn(move || {
            let start = Instant::now();
            let algo: Box<dyn DitherAlgorithm> = match algo_name.as_str() {
                "Bayer 2x2" => Box::new(Bayer2x2),
                "Bayer 4x4" => Box::new(Bayer4x4),
                "Bayer 8x8" => Box::new(Bayer8x8),
                "Bayer 16x16" => Box::new(Bayer16x16),
                "Clustered Dot 4x4" => Box::new(ClusteredDot4x4),
                "Clustered Dot 8x8" => Box::new(ClusteredDot8x8),
                "Interleaved Gradient Noise" => Box::new(InterleavedGradientNoise),
                "Atkinson" => Box::new(Atkinson::default()),
                "Burkes" => Box::new(Burkes::default()),
                "Sierra-3" => Box::new(Sierra3::default()),
                "Sierra-Lite" => Box::new(SierraLite::default()),
                "Stucki" => Box::new(Stucki::default()),
                "Fixed Threshold" => Box::new(FixedThreshold::default()),
                "Adaptive Otsu" => Box::new(OtsuThreshold),
                "Mean Threshold" => Box::new(MeanThreshold),
                "Multi-Level Step" => Box::new(MultiLevelStep::default()),
                "CMYK Halftone" => Box::new(CmykHalftone::default()),
                "Pop Tone" => Box::new(PopTone::default()),
                _ => Box::new(FloydSteinberg::default()),
            };

            let mut out = algo.apply(&source, &palette).unwrap_or_else(|_| (*source).clone());

            // Post-effects
            if let Some(glow) = glow_opt {
                out = glow.apply(&out).unwrap_or(out);
            }
            if let Some(chromatic) = chromatic_opt {
                out = chromatic.apply(&out).unwrap_or(out);
            }
            if let Some(glitch) = glitch_opt {
                out = glitch.apply(&out).unwrap_or(out);
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
            let mut color_data = Vec::with_capacity((res.width * res.height * 4) as usize);
            for chunk in res.data.chunks_exact(3) {
                color_data.push(chunk[0]);
                color_data.push(chunk[1]);
                color_data.push(chunk[2]);
                color_data.push(255);
            }
            let color_image = ColorImage::from_rgba_unmultiplied([res.width as usize, res.height as usize], &color_data);
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

            // Algorithm Picker
            ui.group(|ui| {
                ui.label("Dithering Algorithm:");
                let algorithms = [
                    "Floyd-Steinberg",
                    "Atkinson",
                    "Sierra-3",
                    "Sierra-Lite",
                    "Burkes",
                    "Stucki",
                    "Bayer 2x2",
                    "Bayer 4x4",
                    "Bayer 8x8",
                    "Bayer 16x16",
                    "Clustered Dot 4x4",
                    "Clustered Dot 8x8",
                    "Interleaved Gradient Noise",
                    "Adaptive Otsu",
                    "Fixed Threshold",
                    "Multi-Level Step",
                    "CMYK Halftone",
                    "Pop Tone",
                ];
                egui::ComboBox::from_id_salt("algo_picker").selected_text(&self.selected_algo).show_ui(ui, |ui| {
                    for a in algorithms {
                        if ui.selectable_value(&mut self.selected_algo, a.into(), a).clicked() {
                            changed = true;
                        }
                    }
                });
            });

            // Palette Controls
            ui.group(|ui| {
                ui.label("Target Palette:");
                let palettes = ["Monochrome", "Game Boy", "CGA Mode 1", "Cyberpunk"];
                egui::ComboBox::from_id_salt("palette_picker").selected_text(&self.selected_palette_name).show_ui(ui, |ui| {
                    for p in palettes {
                        if ui.selectable_label(self.selected_palette_name == p, p).clicked() {
                            self.selected_palette_name = p.into();
                            self.active_palette = match p {
                                "Monochrome" => Palette::monochrome(),
                                "CGA Mode 1" => Palette::cga_mode_1(),
                                "Cyberpunk" => Palette::cyberpunk(),
                                _ => Palette::game_boy(),
                            };
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

            // Stackable Effects
            ui.group(|ui| {
                ui.label("Downstream Effects:");
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

        // If temporal noise is running, request repaint
        if self.temporal_noise_enabled {
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
