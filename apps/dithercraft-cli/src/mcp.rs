use dither_core::{
    ImageBuffer,
    algorithms::{ALL_ALGORITHMS, FloydSteinberg, create_algorithm},
    palette::Palette,
    vector::BlackVectorIsolation,
};
use image::ImageReader;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::Path;
use std::time::Instant;

fn load_image(path: &Path) -> Result<ImageBuffer, String> {
    let img = ImageReader::open(path)
        .map_err(|e| format!("Failed to open image: {e}"))?
        .decode()
        .map_err(|e| format!("Failed to decode image: {e}"))?
        .to_rgb8();
    let (width, height) = img.dimensions();
    let data = img.into_raw();
    ImageBuffer::from_raw(width, height, data).map_err(|e| format!("Invalid image buffer: {e}"))
}

fn select_palette(name: &str) -> Palette {
    let p = Path::new(name);
    if p.exists()
        && let Ok(pal) = Palette::import_from_file(p)
    {
        return pal;
    }

    match name.to_lowercase().as_str() {
        "mono" | "monochrome" | "1bit" => Palette::monochrome(),
        "mac" | "mac1bit" => Palette::mac_1bit(),
        "cga" | "cga1" => Palette::cga_mode_1(),
        "cyberpunk" => Palette::cyberpunk(),
        "apple" | "apple2" | "appleii" => Palette::apple_ii(),
        "c64" | "commodore" | "commodore64" => Palette::commodore_64(),
        "nes" => Palette::nes(),
        "pico8" | "pico-8" | "pico" => Palette::pico_8(),
        _ => Palette::game_boy(),
    }
}

pub fn run_mcp_server() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = request.get("id").cloned();
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response: Option<Value> = match method {
            "initialize" => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "dithercraft-mcp",
                        "version": "0.9.0"
                    }
                }
            })),

            "notifications/initialized" => None,

            "tools/list" => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "list_algorithms",
                            "description": "Lists all 63 available dithering and quantization algorithms with their mathematical family categories.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "list_palettes",
                            "description": "Lists all bundled retro hardware palettes with their constituent hex colors.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "dither_image",
                            "description": "Applies a specified dithering algorithm and target palette to an image file, writing the dithered result.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "input": { "type": "string", "description": "Absolute or relative path to the input image" },
                                    "output": { "type": "string", "description": "Path to write the dithered output image (PNG/JPEG)" },
                                    "algo": { "type": "string", "description": "Algorithm name (e.g. 'floyd-steinberg', 'bayer-8x8', 'atkinson', 'riemersma')" },
                                    "palette": { "type": "string", "description": "Target palette name or path to a .json/.gpl/.pal/.hex palette file" }
                                },
                                "required": ["input", "output"]
                            }
                        },
                        {
                            "name": "isolate_vector",
                            "description": "Extracts darkest raster pixel contours into an optimized SVG vector path.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "input": { "type": "string", "description": "Input image path" },
                                    "output": { "type": "string", "description": "Path to write the output SVG" },
                                    "threshold": { "type": "integer", "description": "Maximum luminance threshold (0-255, default: 128)" }
                                },
                                "required": ["input", "output"]
                            }
                        },
                        {
                            "name": "extract_palette",
                            "description": "Extracts representative color palette from an input image using Heckbert Median Cut or Lloyd K-Means clustering.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "input": { "type": "string", "description": "Input image path" },
                                    "num_colors": { "type": "integer", "description": "Number of colors to extract (2-256, default: 16)" },
                                    "method": { "type": "string", "enum": ["median_cut", "kmeans"], "description": "Extraction algorithm (default: 'median_cut')" }
                                },
                                "required": ["input"]
                            }
                        }
                    ]
                }
            })),

            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);

                let tool_result = match tool_name {
                    "list_algorithms" => {
                        let algos: Vec<_> = ALL_ALGORITHMS
                            .iter()
                            .map(|info| {
                                json!({
                                    "name": info.name,
                                    "category": info.category.name(),
                                })
                            })
                            .collect();
                        Ok(json!({ "total": algos.len(), "algorithms": algos }))
                    }

                    "list_palettes" => {
                        let pals = [
                            Palette::monochrome(),
                            Palette::mac_1bit(),
                            Palette::game_boy(),
                            Palette::cga_mode_1(),
                            Palette::cyberpunk(),
                            Palette::apple_ii(),
                            Palette::commodore_64(),
                            Palette::nes(),
                            Palette::pico_8(),
                        ];
                        let pal_list: Vec<_> = pals
                            .iter()
                            .map(|p| {
                                let hexes: Vec<_> =
                                    p.swatches.iter().map(|s| format!("#{:02X}{:02X}{:02X}", s.color.r, s.color.g, s.color.b)).collect();
                                json!({
                                    "name": p.name,
                                    "color_count": p.len(),
                                    "colors": hexes
                                })
                            })
                            .collect();
                        Ok(json!({ "palettes": pal_list }))
                    }

                    "dither_image" => {
                        let input_path = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                        let output_path = args.get("output").and_then(|v| v.as_str()).unwrap_or("");
                        let algo_name = args.get("algo").and_then(|v| v.as_str()).unwrap_or("floyd-steinberg");
                        let palette_name = args.get("palette").and_then(|v| v.as_str()).unwrap_or("gameboy");

                        if input_path.is_empty() || output_path.is_empty() {
                            Err("Both 'input' and 'output' parameters are required".to_string())
                        } else {
                            let start = Instant::now();
                            let img = load_image(Path::new(input_path))?;
                            let pal = select_palette(palette_name);
                            let algorithm = create_algorithm(algo_name).unwrap_or_else(|| Box::new(FloydSteinberg::default()));

                            let dithered = algorithm.apply(&img, &pal).map_err(|e| format!("Dithering failed: {e}"))?;

                            image::save_buffer(output_path, &dithered.data, dithered.width, dithered.height, image::ExtendedColorType::Rgb8)
                                .map_err(|e| format!("Failed to save output: {e}"))?;

                            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                            Ok(json!({
                                "status": "success",
                                "algorithm": algorithm.name(),
                                "palette": pal.name,
                                "dimensions": [dithered.width, dithered.height],
                                "duration_ms": elapsed_ms,
                                "output": output_path
                            }))
                        }
                    }

                    "isolate_vector" => {
                        let input_path = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                        let output_path = args.get("output").and_then(|v| v.as_str()).unwrap_or("");
                        let threshold = args.get("threshold").and_then(|v| v.as_u64()).unwrap_or(128) as u8;

                        if input_path.is_empty() || output_path.is_empty() {
                            Err("Both 'input' and 'output' parameters are required".to_string())
                        } else {
                            let start = Instant::now();
                            let img = load_image(Path::new(input_path))?;
                            let isolator = BlackVectorIsolation { max_luminance_threshold: threshold, pixel_size: 1.0 };
                            let svg = isolator.trace_to_svg(&img).map_err(|e| format!("Vector tracing failed: {e}"))?;

                            std::fs::write(output_path, svg).map_err(|e| format!("Failed to save SVG: {e}"))?;

                            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                            Ok(json!({
                                "status": "success",
                                "output": output_path,
                                "duration_ms": elapsed_ms
                            }))
                        }
                    }

                    "extract_palette" => {
                        let input_path = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                        let num_colors = args.get("num_colors").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
                        let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("median_cut");

                        if input_path.is_empty() {
                            Err("'input' parameter is required".to_string())
                        } else {
                            let img = load_image(Path::new(input_path))?;
                            let pal = if method.eq_ignore_ascii_case("kmeans") {
                                Palette::from_image_kmeans("Extracted KMeans", &img, num_colors, 10)
                                    .map_err(|e| format!("KMeans extraction failed: {e}"))?
                            } else {
                                Palette::from_image_median_cut("Extracted MedianCut", &img, num_colors)
                                    .map_err(|e| format!("MedianCut extraction failed: {e}"))?
                            };

                            let hexes: Vec<_> =
                                pal.swatches.iter().map(|s| format!("#{:02X}{:02X}{:02X}", s.color.r, s.color.g, s.color.b)).collect();

                            Ok(json!({
                                "status": "success",
                                "method": method,
                                "count": hexes.len(),
                                "colors": hexes
                            }))
                        }
                    }

                    _ => Err(format!("Unknown tool: {tool_name}")),
                };

                let response_content = match tool_result {
                    Ok(val) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": val.to_string()
                            }],
                            "isError": false
                        }
                    }),
                    Err(err_msg) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": err_msg
                            }],
                            "isError": true
                        }
                    }),
                };

                Some(response_content)
            }

            _ => {
                if id.is_some() {
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {method}")
                        }
                    }))
                } else {
                    None
                }
            }
        };

        if let Some(res) = response {
            writeln!(stdout, "{}", res)?;
            stdout.flush()?;
        }
    }

    Ok(())
}
