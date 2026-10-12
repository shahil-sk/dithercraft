use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::{AnimationDecoder, Frame, RgbaImage};

#[test]
fn test_animated_gif_generation_and_reading() {
    let tmp_dir = std::env::temp_dir();
    let gif_path = tmp_dir.join("test_anim_src.gif");

    // Create a 2-frame source GIF
    let file = std::fs::File::create(&gif_path).expect("create file");
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(Repeat::Infinite).expect("set repeat");

    for i in 0..2 {
        let mut rgba = Vec::new();
        for _ in 0..(16 * 16) {
            rgba.push((i * 100) as u8);
            rgba.push(120);
            rgba.push(200);
            rgba.push(255);
        }
        let img = RgbaImage::from_raw(16, 16, rgba).expect("raw image");
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(100));
        let frame = Frame::from_parts(img, 0, 0, delay);
        encoder.encode_frame(frame).expect("encode frame");
    }
    drop(encoder);

    // Read back using GifDecoder
    let file_in = std::fs::File::open(&gif_path).expect("open file");
    let decoder = GifDecoder::new(std::io::BufReader::new(file_in)).expect("decoder");
    let frames = decoder.into_frames().collect_frames().expect("collect frames");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].buffer().width(), 16);
    assert_eq!(frames[0].buffer().height(), 16);

    let _ = std::fs::remove_file(&gif_path);
}

#[test]
fn test_apng_generation_and_reading() {
    let tmp_dir = std::env::temp_dir();
    let apng_path = tmp_dir.join("test_anim_src.png");

    let file = std::fs::File::create(&apng_path).expect("create apng file");
    let mut writer_buf = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(&mut writer_buf, 16, 16);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(2, 0).expect("set animated");
    encoder.set_frame_delay(100, 1000).expect("set frame delay");
    let mut writer = encoder.write_header().expect("write header");

    for i in 0..2 {
        let mut rgba = Vec::new();
        for _ in 0..(16 * 16) {
            rgba.push((i * 100) as u8);
            rgba.push(120);
            rgba.push(200);
            rgba.push(255);
        }
        writer.set_frame_delay(100, 1000).expect("frame delay");
        writer.write_image_data(&rgba).expect("write frame");
    }
    drop(writer);
    drop(writer_buf);

    assert!(apng_path.exists());
    let metadata = std::fs::metadata(&apng_path).expect("metadata");
    assert!(metadata.len() > 0);

    let _ = std::fs::remove_file(&apng_path);
}
