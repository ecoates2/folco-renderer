//! Hue rotation layer configuration and application.

use super::{DependencyVersion, DominantColor, LayerConfig, LayerEffect, LayerVersions, RenderContext};
use crate::icon::IconImage;
use palette::{Hsl, IntoColor, Srgb};

// ============================================================================
// HueRotationConfig
// ============================================================================

/// Configuration for hue rotation.
///
/// Rotates the hue of all pixels by a specified number of degrees.
///
/// # Emitted Properties
///
/// - [`DominantColor`]: The dominant color sampled after hue rotation.
#[derive(Debug, Clone)]
pub struct HueRotationConfig {
    /// Rotation angle in degrees (0-360).
    pub degrees: f32,
}

impl HueRotationConfig {
    /// Creates a new hue rotation config with the given angle.
    ///
    /// The angle is normalized to the 0-360 range.
    pub fn new(degrees: f32) -> Self {
        Self {
            degrees: degrees.rem_euclid(360.0),
        }
    }
}

impl LayerConfig for HueRotationConfig {
    fn differs_from(&self, other: &Self) -> bool {
        (self.degrees - other.degrees).abs() > 0.001
    }
}

impl LayerEffect for HueRotationConfig {
    /// Hue rotation has no upstream dependencies (root layer).
    fn dependencies(_versions: &LayerVersions) -> DependencyVersion {
        DependencyVersion::NONE
    }

    fn transform(&self, ctx: &mut RenderContext) {
        ctx.image = apply_hue_rotation(&ctx.image, self);
    }

    fn emit(&self, ctx: &mut RenderContext) {
        // Emit dominant color for downstream layers (e.g., decal)
        let color = sample_dominant_color(&ctx.image);
        ctx.set(DominantColor::new(color.0, color.1, color.2, color.3));
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Applies hue rotation to an icon image.
pub fn apply_hue_rotation(icon: &IconImage, config: &HueRotationConfig) -> IconImage {
    let mut result = icon.data.clone();

    for pixel in result.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if a == 0 {
            continue; // Skip fully transparent pixels
        }

        // Convert to HSL, rotate hue, convert back
        let rgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let mut hsl: Hsl = rgb.into_color();
        hsl.hue += config.degrees;
        let rotated: Srgb = hsl.into_color();

        pixel.0 = [
            (rotated.red * 255.0).round() as u8,
            (rotated.green * 255.0).round() as u8,
            (rotated.blue * 255.0).round() as u8,
            a,
        ];
    }

    IconImage::new(result, icon.scale, icon.content_bounds)
}

/// Samples the dominant/average color from the icon's content bounds.
pub fn sample_dominant_color(icon: &IconImage) -> (u8, u8, u8, u8) {
    let bounds = icon.content_bounds;
    let img = &icon.data;

    let mut total_r: u64 = 0;
    let mut total_g: u64 = 0;
    let mut total_b: u64 = 0;
    let mut total_a: u64 = 0;
    let mut count: u64 = 0;

    for y in bounds.y..bounds.bottom().min(img.height()) {
        for x in bounds.x..bounds.right().min(img.width()) {
            let pixel = img.get_pixel(x, y);
            let a = pixel[3] as u64;
            if a > 0 {
                // Weight by alpha for proper averaging
                total_r += pixel[0] as u64 * a;
                total_g += pixel[1] as u64 * a;
                total_b += pixel[2] as u64 * a;
                total_a += a;
                count += 1;
            }
        }
    }

    if count == 0 || total_a == 0 {
        return (128, 128, 128, 255); // Default gray if no pixels
    }

    (
        (total_r / total_a) as u8,
        (total_g / total_a) as u8,
        (total_b / total_a) as u8,
        (total_a / count) as u8,
    )
}
