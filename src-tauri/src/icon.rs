use tauri::image::Image;

const SIZE: u32 = 32;

/// Render a 32×32 RGBA horizontal progress bar tray icon.
///
/// The inner 30×30 area is split into a filled portion (left) and a dark
/// background (right), with a 1px gray border on all sides. The filled
/// width maps linearly to `percent` (0.0–100.0).
pub fn render(percent: f64, hex_color: &str) -> Image<'static> {
    let (fr, fg, fb) = parse_hex(hex_color).unwrap_or((0x2E, 0xCC, 0x71));
    let inner = SIZE - 2; // 30 pixels of inner width
    let filled = (percent.clamp(0.0, 100.0) / 100.0 * inner as f64).round() as u32;

    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            let border = x == 0 || x == SIZE - 1 || y == 0 || y == SIZE - 1;

            let (r, g, b) = if border {
                (0x55, 0x55, 0x55) // 1px gray border
            } else if x - 1 < filled {
                (fr, fg, fb)       // filled portion
            } else {
                (0x22, 0x22, 0x22) // dark background
            };

            pixels[i]     = r;
            pixels[i + 1] = g;
            pixels[i + 2] = b;
            pixels[i + 3] = 0xFF;
        }
    }

    Image::new_owned(pixels, SIZE, SIZE)
}

/// Parse a `#RRGGBB` hex string into `(r, g, b)` bytes. Returns `None` on
/// any format error so the caller can fall back to a safe default.
pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let s = hex.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_hex ────────────────────────────────────────────────────────────

    #[test]
    fn parse_hex_green() {
        assert_eq!(parse_hex("#2ECC71"), Some((0x2E, 0xCC, 0x71)));
    }

    #[test]
    fn parse_hex_missing_hash_returns_none() {
        assert_eq!(parse_hex("2ECC71"), None);
    }

    #[test]
    fn parse_hex_wrong_length_returns_none() {
        assert_eq!(parse_hex("#FFF"), None);
    }

    // ── render: pixel geometry ───────────────────────────────────────────────

    fn pixel(buf: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
        let i = ((y * SIZE + x) * 4) as usize;
        (buf[i], buf[i + 1], buf[i + 2], buf[i + 3])
    }

    #[test]
    fn render_produces_correct_dimensions() {
        let img = render(50.0, "#2ECC71");
        assert_eq!(img.width(),  SIZE);
        assert_eq!(img.height(), SIZE);
        assert_eq!(img.rgba().len() as u32, SIZE * SIZE * 4);
    }

    #[test]
    fn border_pixels_are_gray() {
        let img = render(50.0, "#2ECC71");
        let buf = img.rgba();
        // top-left corner
        assert_eq!(pixel(buf, 0, 0), (0x55, 0x55, 0x55, 0xFF));
        // bottom-right corner
        assert_eq!(pixel(buf, SIZE - 1, SIZE - 1), (0x55, 0x55, 0x55, 0xFF));
    }

    #[test]
    fn zero_percent_renders_no_filled_pixels() {
        let img = render(0.0, "#2ECC71");
        let buf = img.rgba();
        // Inner leftmost column (x=1) should be background, not fill color
        assert_eq!(pixel(buf, 1, 1), (0x22, 0x22, 0x22, 0xFF));
    }

    #[test]
    fn full_percent_renders_all_filled() {
        let img = render(100.0, "#2ECC71");
        let buf = img.rgba();
        // Inner rightmost column (x=SIZE-2) should be filled
        assert_eq!(pixel(buf, SIZE - 2, 1), (0x2E, 0xCC, 0x71, 0xFF));
    }

    #[test]
    fn fifty_percent_fills_half_inner_width() {
        let img = render(50.0, "#2ECC71");
        let buf = img.rgba();
        let inner = SIZE - 2; // 30
        let half  = inner / 2; // 15
        // Column at the boundary — x = half (0-indexed inner = half-1, outer = half)
        assert_eq!(pixel(buf, half, 1), (0x2E, 0xCC, 0x71, 0xFF));       // last filled
        assert_eq!(pixel(buf, half + 1, 1), (0x22, 0x22, 0x22, 0xFF));   // first empty
    }

    #[test]
    fn all_pixels_fully_opaque() {
        let img = render(42.0, "#F1C40F");
        let all_opaque = img.rgba().chunks(4).all(|p| p[3] == 0xFF);
        assert!(all_opaque, "every pixel should be fully opaque");
    }
}
