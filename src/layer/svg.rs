//! SVG rendering utilities using resvg/usvg.
//!
//! This module provides shared SVG parsing and rendering functionality
//! used by both the decal and overlay layers.

use image::{Rgba, RgbaImage};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

// ============================================================================
// SvgSource
// ============================================================================

/// A source for SVG data.
///
/// This enum allows layers to accept SVG content from multiple sources:
/// - Raw SVG markup strings
/// - Emoji characters (when the `twemoji` feature is enabled)
///
/// # Example
///
/// ```
/// use folco_renderer::SvgSource;
///
/// // From raw SVG
/// let raw = SvgSource::from_svg("<svg>...</svg>");
///
/// // From emoji (requires `twemoji` feature)
/// #[cfg(feature = "twemoji")]
/// let emoji = SvgSource::from_emoji("🦆").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgSource {
    /// Raw SVG markup string.
    Raw(String),

    /// An emoji character to be resolved via twemoji_assets.
    ///
    /// Only available when the `twemoji` feature is enabled.
    /// At render time, this is resolved to the corresponding Twemoji SVG.
    Emoji(String),
}

impl SvgSource {
    /// Creates a source from raw SVG markup.
    pub fn from_svg(svg: impl Into<String>) -> Self {
        Self::Raw(svg.into())
    }

    /// Creates a source from an emoji character.
    ///
    /// Returns `None` if the emoji is not supported by twemoji_assets.
    /// Only available when the `twemoji` feature is enabled.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji(emoji: &str) -> Option<Self> {
        use twemoji_assets::svg::SvgTwemojiAsset;

        // Validate that the emoji exists
        SvgTwemojiAsset::from_emoji(emoji)?;
        Some(Self::Emoji(emoji.to_string()))
    }

    /// Resolves this source to SVG markup.
    ///
    /// For `Raw` sources, returns the SVG string directly.
    /// For `Emoji` sources, looks up the emoji in twemoji_assets.
    pub fn resolve(&self) -> Option<&str> {
        match self {
            Self::Raw(svg) => Some(svg.as_str()),
            #[cfg(feature = "twemoji")]
            Self::Emoji(emoji) => {
                use twemoji_assets::svg::SvgTwemojiAsset;
                let asset = SvgTwemojiAsset::from_emoji(emoji)?;
                Some(asset.as_ref())
            }
            #[cfg(not(feature = "twemoji"))]
            Self::Emoji(_) => None,
        }
    }

    /// Returns `true` if this is an emoji source.
    pub fn is_emoji(&self) -> bool {
        matches!(self, Self::Emoji(_))
    }

    /// Returns `true` if this is a raw SVG source.
    pub fn is_raw(&self) -> bool {
        matches!(self, Self::Raw(_))
    }
}

impl<S: Into<String>> From<S> for SvgSource {
    fn from(s: S) -> Self {
        Self::Raw(s.into())
    }
}

// ============================================================================
// SVG Rendering
// ============================================================================

/// Renders an SVG string to an RGBA image at the specified size.
///
/// The SVG is scaled to fit within `size x size` pixels while preserving
/// aspect ratio (the larger dimension will be `size`).
///
/// Returns `None` if the SVG cannot be parsed or rendered.
pub fn render_svg(svg_data: &str, size: u32) -> Option<RgbaImage> {
    render_svg_with_color(svg_data, size, None)
}

/// Renders an SVG string to an RGBA image, optionally replacing all colors.
///
/// If `fill_color` is provided, all fills and strokes in the SVG are replaced
/// with this color. This is useful for monochrome icon decals.
///
/// Returns `None` if the SVG cannot be parsed or rendered.
pub fn render_svg_with_color(
    svg_data: &str,
    size: u32,
    fill_color: Option<(u8, u8, u8, u8)>,
) -> Option<RgbaImage> {
    // Apply color replacement if needed
    let svg_data = if let Some((r, g, b, _a)) = fill_color {
        replace_svg_colors(svg_data, r, g, b)
    } else {
        svg_data.to_string()
    };

    // Parse the SVG
    let opts = Options::default();
    let tree = Tree::from_str(&svg_data, &opts).ok()?;

    // Calculate scale to fit within size x size
    let svg_size = tree.size();
    let scale = (size as f32) / svg_size.width().max(svg_size.height());
    let width = (svg_size.width() * scale).ceil() as u32;
    let height = (svg_size.height() * scale).ceil() as u32;

    // Create pixmap and render
    let mut pixmap = Pixmap::new(width, height)?;
    let transform = Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert to RgbaImage
    Some(pixmap_to_rgba_image(&pixmap))
}

/// Renders an [`SvgSource`] to an RGBA image at the specified size.
///
/// This is a convenience wrapper around [`render_svg`] that handles source resolution.
pub fn render_source(source: &SvgSource, size: u32) -> Option<RgbaImage> {
    let svg_data = source.resolve()?;
    render_svg(svg_data, size)
}

/// Renders an [`SvgSource`] to an RGBA image, optionally replacing all colors.
///
/// This is a convenience wrapper around [`render_svg_with_color`] that handles source resolution.
pub fn render_source_with_color(
    source: &SvgSource,
    size: u32,
    fill_color: Option<(u8, u8, u8, u8)>,
) -> Option<RgbaImage> {
    let svg_data = source.resolve()?;
    render_svg_with_color(svg_data, size, fill_color)
}

/// Replaces common color attributes in SVG with the specified RGB color.
///
/// This is a simple text-based replacement that handles common cases:
/// - `fill="..."` attributes
/// - `stroke="..."` attributes
/// - `style="..."` attributes containing fill/stroke
///
/// For complex SVGs, consider using a proper SVG manipulation library.
fn replace_svg_colors(svg_data: &str, r: u8, g: u8, b: u8) -> String {
    let hex_color = format!("#{:02x}{:02x}{:02x}", r, g, b);

    // Replace fill and stroke attributes
    // This is a simple approach; for production, consider using an XML parser
    let mut result = svg_data.to_string();

    // Replace fill="..." (but not fill="none")
    result = replace_color_attr(&result, "fill", &hex_color);
    // Replace stroke="..." (but not stroke="none")
    result = replace_color_attr(&result, "stroke", &hex_color);

    result
}

/// Replaces a color attribute value, preserving "none" values.
fn replace_color_attr(svg: &str, attr: &str, new_color: &str) -> String {
    let mut result = String::with_capacity(svg.len());
    let pattern = format!("{}=\"", attr);
    let mut remaining = svg;

    while let Some(start) = remaining.find(&pattern) {
        // Copy everything up to and including the attribute name and ="
        result.push_str(&remaining[..start + pattern.len()]);
        remaining = &remaining[start + pattern.len()..];

        // Find the closing quote
        if let Some(end) = remaining.find('"') {
            let value = &remaining[..end];
            // Preserve "none" and "transparent", replace everything else
            if value == "none" || value == "transparent" {
                result.push_str(value);
            } else {
                result.push_str(new_color);
            }
            remaining = &remaining[end..];
        }
    }

    // Append any remaining content
    result.push_str(remaining);
    result
}

/// Converts a tiny_skia Pixmap to an image::RgbaImage.
fn pixmap_to_rgba_image(pixmap: &Pixmap) -> RgbaImage {
    let width = pixmap.width();
    let height = pixmap.height();
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let pixel = pixmap.pixel(x, y).unwrap();
            // tiny_skia uses premultiplied alpha, we need to unpremultiply
            let (r, g, b, a) = unpremultiply(pixel.red(), pixel.green(), pixel.blue(), pixel.alpha());
            img.put_pixel(x, y, Rgba([r, g, b, a]));
        }
    }

    img
}

/// Unpremultiplies a premultiplied alpha pixel.
fn unpremultiply(r: u8, g: u8, b: u8, a: u8) -> (u8, u8, u8, u8) {
    if a == 0 {
        (0, 0, 0, 0)
    } else {
        let a_f = a as f32 / 255.0;
        (
            (r as f32 / a_f).round().min(255.0) as u8,
            (g as f32 / a_f).round().min(255.0) as u8,
            (b as f32 / a_f).round().min(255.0) as u8,
            a,
        )
    }
}

// ============================================================================
// Compositing
// ============================================================================

/// Composites a source image onto a destination image at the specified position.
///
/// Uses standard alpha blending (source over destination).
pub fn composite_over(dest: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    let dest_width = dest.width() as i32;
    let dest_height = dest.height() as i32;

    for sy in 0..src.height() {
        for sx in 0..src.width() {
            let dx = x + sx as i32;
            let dy = y + sy as i32;

            // Skip if outside destination bounds
            if dx < 0 || dy < 0 || dx >= dest_width || dy >= dest_height {
                continue;
            }

            let src_pixel = src.get_pixel(sx, sy);
            let dst_pixel = dest.get_pixel(dx as u32, dy as u32);

            // Alpha blending (source over)
            let blended = alpha_blend(*src_pixel, *dst_pixel);
            dest.put_pixel(dx as u32, dy as u32, blended);
        }
    }
}

/// Alpha blends two RGBA pixels (source over destination).
fn alpha_blend(src: Rgba<u8>, dst: Rgba<u8>) -> Rgba<u8> {
    let sa = src[3] as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;

    // Source over compositing
    let out_a = sa + da * (1.0 - sa);

    if out_a == 0.0 {
        return Rgba([0, 0, 0, 0]);
    }

    let blend = |s: u8, d: u8| -> u8 {
        let sf = s as f32 / 255.0;
        let df = d as f32 / 255.0;
        let out = (sf * sa + df * da * (1.0 - sa)) / out_a;
        (out * 255.0).round() as u8
    };

    Rgba([
        blend(src[0], dst[0]),
        blend(src[1], dst[1]),
        blend(src[2], dst[2]),
        (out_a * 255.0).round() as u8,
    ])
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><circle cx="50" cy="50" r="40" fill="#ff0000"/></svg>"##;

    #[test]
    fn render_simple_svg() {
        let result = render_svg(SIMPLE_SVG, 50);
        assert!(result.is_some());

        let img = result.unwrap();
        assert!(img.width() <= 50);
        assert!(img.height() <= 50);
    }

    #[test]
    fn render_svg_with_color_replacement() {
        let result = render_svg_with_color(SIMPLE_SVG, 50, Some((0, 255, 0, 255)));
        assert!(result.is_some());

        let img = result.unwrap();
        // Check that the center pixel (inside the circle) is green-ish
        let center = img.get_pixel(img.width() / 2, img.height() / 2);
        assert!(center[1] > center[0], "Green should dominate after color replacement");
    }

    #[test]
    fn composite_simple() {
        // Create a 10x10 red background
        let mut dest = RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255]));

        // Create a 4x4 blue overlay
        let src = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 255]));

        // Composite at (3, 3)
        composite_over(&mut dest, &src, 3, 3);

        // Check that the overlay area is blue
        assert_eq!(dest.get_pixel(5, 5).0, [0, 0, 255, 255]);

        // Check that outside the overlay is still red
        assert_eq!(dest.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn composite_with_transparency() {
        // Create a 10x10 red background
        let mut dest = RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255]));

        // Create a 4x4 semi-transparent blue overlay
        let src = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 255, 128]));

        // Composite at (0, 0)
        composite_over(&mut dest, &src, 0, 0);

        // The result should be a blend of red and blue
        let pixel = dest.get_pixel(0, 0);
        assert!(pixel[0] > 0, "Should have some red");
        assert!(pixel[2] > 0, "Should have some blue");
    }

    #[test]
    fn replace_color_preserves_none() {
        let svg = r##"<circle fill="none" stroke="#000000"/>"##;
        let result = replace_svg_colors(svg, 255, 0, 0);
        assert!(result.contains(r#"fill="none""#));
        assert!(result.contains(r##"stroke="#ff0000""##));
    }

    #[test]
    fn svg_source_from_raw() {
        let source = SvgSource::from_svg("<svg></svg>");
        assert!(source.is_raw());
        assert!(!source.is_emoji());
        assert_eq!(source.resolve(), Some("<svg></svg>"));
    }

    #[test]
    fn svg_source_into_from_string() {
        let source: SvgSource = "<svg></svg>".into();
        assert!(source.is_raw());
        assert_eq!(source.resolve(), Some("<svg></svg>"));
    }

    #[cfg(feature = "twemoji")]
    #[test]
    fn svg_source_from_emoji() {
        let source = SvgSource::from_emoji("🦆").expect("Duck emoji should be supported");
        assert!(source.is_emoji());
        assert!(!source.is_raw());
        
        let svg = source.resolve().expect("Should resolve to SVG");
        assert!(svg.contains("<svg"), "Should be valid SVG data");
    }

    #[cfg(feature = "twemoji")]
    #[test]
    fn svg_source_invalid_emoji_returns_none() {
        // An invalid/unsupported string should return None
        let source = SvgSource::from_emoji("not-an-emoji");
        assert!(source.is_none());
    }

    #[cfg(feature = "twemoji")]
    #[test]
    fn render_emoji_source() {
        let source = SvgSource::from_emoji("🦆").unwrap();
        let img = render_source(&source, 64);
        assert!(img.is_some(), "Should render emoji to image");
        
        let img = img.unwrap();
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }
}
