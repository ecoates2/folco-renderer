//! HSL mutation layer configuration and application.
//!
//! Implements GIMP-style Hue/Saturation adjustment: hue is shifted by a
//! delta in degrees, while saturation and lightness are scaled by a
//! multiplicative factor derived from the delta value.
//!
//! # GIMP Hue-Saturation algorithm
//!
//! For each pixel the operation converts to HSL, then:
//!
//! - **Hue**: `new_hue = old_hue + hue_shift`
//! - **Saturation**: `new_saturation = old_saturation * (1.0 + saturation_delta)`
//! - **Lightness**: `new_lightness = old_lightness * (1.0 + lightness_delta)`
//!
//! Where `saturation_delta` and `lightness_delta` are in the range \[-1.0, 1.0\].
//!
//! A delta of 0.0 leaves the channel unchanged, +1.0 doubles it,
//! and -1.0 drives it to zero.

use super::{DependencyVersion, DominantColor, LayerConfig, LayerEffect, LayerVersions, RenderContext};
use crate::error::RenderError;
use crate::icon::{IconImage, SurfaceColor};
use palette::{Hsl, IntoColor, Srgb};

// ============================================================================
// HslMutationConfig
// ============================================================================

/// Configuration for HSL mutation.
///
/// Adjusts hue, saturation, and lightness of all pixels using the same
/// algorithm as GIMP's Hue/Saturation tool.
///
/// # Construction
///
/// Use [`new`](Self::new) which accepts a target HSL color and a base
/// surface color. The necessary internal deltas are computed automatically.
///
/// # Emitted Properties
///
/// - [`DominantColor`]: The target color is always emitted directly,
///   avoiding expensive per-pixel sampling.
#[derive(Debug, Clone)]
pub struct HslMutationConfig {
    /// Target hue in degrees (0–360).
    pub target_hue: f32,
    /// Target saturation (0.0–1.0).
    pub target_saturation: f32,
    /// Target lightness (0.0–1.0).
    pub target_lightness: f32,
    /// Hue shift in degrees, computed from (target_hue − surface_hue).
    hue_shift: f32,
    /// Saturation scale delta, computed from target/surface ratio.
    saturation_delta: f32,
    /// Lightness scale delta, computed from target/surface ratio.
    lightness_delta: f32,
    /// Pre-computed target color (RGB) for direct DominantColor emission.
    target_color: (u8, u8, u8),
}

impl HslMutationConfig {
    /// Creates a new HSL mutation config from a target HSL color and surface color.
    ///
    /// Computes the internal deltas needed to transform the surface color into the
    /// target color using the GIMP Hue/Saturation algorithm:
    ///
    /// - `hue_shift = target_hue − surface_hue`
    /// - `saturation_delta = (target_sat / surface_sat) − 1.0`
    /// - `lightness_delta = (target_light / surface_light) − 1.0`
    ///
    /// Target values are normalized/clamped:
    /// - `target_hue` to 0–360
    /// - `target_saturation` and `target_lightness` to 0.0–1.0
    pub fn new(
        surface: &SurfaceColor,
        target_hue: f32,
        target_saturation: f32,
        target_lightness: f32,
    ) -> Self {
        let target_hue = target_hue.rem_euclid(360.0);
        let target_saturation = target_saturation.clamp(0.0, 1.0);
        let target_lightness = target_lightness.clamp(0.0, 1.0);

        let hue_shift = (target_hue - surface.hue).rem_euclid(360.0);
        let saturation_delta = if surface.saturation > 0.0 {
            (target_saturation / surface.saturation) - 1.0
        } else {
            0.0
        };
        let lightness_delta = if surface.lightness > 0.0 {
            (target_lightness / surface.lightness) - 1.0
        } else {
            0.0
        };

        // Convert target HSL to RGB for direct emission
        let target_hsl = Hsl::new(target_hue, target_saturation, target_lightness);
        let target_rgb: Srgb = target_hsl.into_color();
        let target_color = (
            (target_rgb.red * 255.0).round() as u8,
            (target_rgb.green * 255.0).round() as u8,
            (target_rgb.blue * 255.0).round() as u8,
        );

        Self {
            target_hue,
            target_saturation,
            target_lightness,
            hue_shift,
            saturation_delta: saturation_delta.clamp(-1.0, 1.0),
            lightness_delta: lightness_delta.clamp(-1.0, 1.0),
            target_color,
        }
    }

    /// Creates a config that only changes the hue, keeping the surface's
    /// saturation and lightness.
    pub fn hue_only(surface: &SurfaceColor, target_hue: f32) -> Self {
        Self::new(surface, target_hue, surface.saturation, surface.lightness)
    }
}

impl LayerConfig for HslMutationConfig {
    fn differs_from(&self, other: &Self) -> bool {
        (self.target_hue - other.target_hue).abs() > 0.001
            || (self.target_saturation - other.target_saturation).abs() > 0.001
            || (self.target_lightness - other.target_lightness).abs() > 0.001
    }
}

impl LayerEffect for HslMutationConfig {
    /// HSL mutation has no upstream dependencies (root layer).
    fn dependencies(_versions: &LayerVersions) -> DependencyVersion {
        DependencyVersion::NONE
    }

    fn transform(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
        ctx.image = apply_hsl_mutation(&ctx.image, self);
        Ok(())
    }

    fn emit(&self, ctx: &mut RenderContext) {
        let (r, g, b) = self.target_color;
        ctx.set(DominantColor::new(r, g, b, 255));
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Applies GIMP-style HSL mutation to an icon image.
///
/// For each opaque pixel the operation:
/// 1. Converts sRGB → HSL
/// 2. Shifts hue by `config.hue_shift` degrees
/// 3. Scales saturation by `1.0 + config.saturation_delta`
/// 4. Scales lightness by `1.0 + config.lightness_delta`
/// 5. Clamps S and L to \[0.0, 1.0\]
/// 6. Converts back to sRGB
pub fn apply_hsl_mutation(icon: &IconImage, config: &HslMutationConfig) -> IconImage {
    let mut result = icon.data.clone();

    let sat_factor = 1.0 + config.saturation_delta;
    let light_factor = 1.0 + config.lightness_delta;

    for pixel in result.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if a == 0 {
            continue; // Skip fully transparent pixels
        }

        // Convert to HSL
        let rgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let mut hsl: Hsl = rgb.into_color();

        // Shift hue
        hsl.hue += config.hue_shift;

        // Scale saturation (GIMP-style: multiply by 1 + delta)
        hsl.saturation = (hsl.saturation * sat_factor).clamp(0.0, 1.0);

        // Scale lightness (GIMP-style: multiply by 1 + delta)
        hsl.lightness = (hsl.lightness * light_factor).clamp(0.0, 1.0);

        let mutated: Srgb = hsl.into_color();

        pixel.0 = [
            (mutated.red * 255.0).round() as u8,
            (mutated.green * 255.0).round() as u8,
            (mutated.blue * 255.0).round() as u8,
            a,
        ];
    }

    IconImage::new(result, icon.scale, icon.content_bounds)
}

