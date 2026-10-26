//! Unit tests for data/clip_importer.rs.

use super::*;
use image::{Delay, Rgba};

/// A white frame with a filled red circle, like a cat filmed against a plain wall.
fn circle_on_white(w: u32, h: u32, cx: f64, cy: f64, r: f64) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        if (x as f64 - cx).hypot(y as f64 - cy) < r { Rgba([200, 30, 30, 255]) } else { Rgba([250, 250, 250, 255]) }
    })
}

#[test]
fn alpha_bounds_find_the_visible_pixels() {
    let mut img = RgbaImage::new(20, 10);
    assert_eq!(alpha_bounds(&img), None);
    img.put_pixel(3, 4, Rgba([0, 0, 0, 255]));
    img.put_pixel(7, 6, Rgba([0, 0, 0, 255]));
    img.put_pixel(15, 1, Rgba([0, 0, 0, 10])); // too faint to count
    assert_eq!(alpha_bounds(&img), Some(PixelBox { x: 3, y: 4, w: 5, h: 3 }));
}

#[test]
fn boxes_union() {
    let a = PixelBox { x: 0, y: 5, w: 10, h: 2 };
    let b = PixelBox { x: 5, y: 0, w: 10, h: 3 };
    assert_eq!(a.union(&b), PixelBox { x: 0, y: 0, w: 15, h: 7 });
    assert_eq!(a.mid_x(), 5.0);
}

#[test]
fn removes_a_plain_background() {
    let out = remove_plain_background(&circle_on_white(80, 60, 40.0, 30.0, 15.0)).unwrap();
    assert_eq!(out.get_pixel(2, 2)[3], 0, "the wall is gone");
    assert_eq!(out.get_pixel(40, 30)[3], 255, "the subject stays");
    let b = alpha_bounds(&out).unwrap();
    assert!(b.w >= 28 && b.w <= 31 && b.h >= 28 && b.h <= 31, "{b:?}");
}

#[test]
fn drops_specks_but_keeps_big_pieces() {
    let mut img = circle_on_white(100, 60, 30.0, 30.0, 15.0);
    for (x, y) in [(80, 10), (81, 10)] { img.put_pixel(x, y, Rgba([0, 0, 0, 255])) }
    let out = remove_plain_background(&img).unwrap();
    assert_eq!(out.get_pixel(80, 10)[3], 0);
}

#[test]
fn gives_up_without_a_subject() {
    assert!(remove_plain_background(&RgbaImage::from_pixel(40, 40, Rgba([250, 250, 250, 255]))).is_none());
    assert!(remove_plain_background(&RgbaImage::new(2, 2)).is_none());
}

#[test]
fn frame_delays_default_when_missing() {
    let f = Frame::from_parts(RgbaImage::new(1, 1), 0, 0, Delay::from_numer_denom_ms(50, 1));
    assert_eq!(frame_delay(&f), 0.05);
    let f = Frame::from_parts(RgbaImage::new(1, 1), 0, 0, Delay::from_numer_denom_ms(0, 1));
    assert_eq!(frame_delay(&f), 0.08);
}

#[test]
fn imports_an_animated_gif_as_a_walking_clip() {
    let dir = std::env::temp_dir().join(format!("pawpal-gif-{}", fastrand::u64(..)));
    std::fs::create_dir_all(&dir).unwrap();
    let gif = dir.join("walk.gif");
    {
        let mut enc = image::codecs::gif::GifEncoder::new(File::create(&gif).unwrap());
        let frames = (0..6).map(|i| Frame::from_parts(circle_on_white(120, 60, 25.0 + 14.0 * i as f64, 30.0, 12.0), 0, 0, Delay::from_numer_denom_ms(100, 1)));
        enc.encode_frames(frames).unwrap();
    }
    let (frames, fps) = read_frames(&gif).unwrap();
    assert_eq!(frames.len(), 6);
    assert!((fps - 10.0).abs() < 0.01);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unreadable_files_are_errors() {
    assert!(read_frames(Path::new("/nonexistent/clip.gif")).is_err());
}
