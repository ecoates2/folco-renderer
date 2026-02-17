//! Decal imprint layer configuration and application.

use super::svg::{composite_over, render_source_with_color, SvgSource};
use super::{DependencyVersion, DominantColor, LayerConfig, LayerEffect, LayerVersions, RenderContext};
use crate::error::RenderError;
use crate::icon::{IconImage, SurfaceColor};
use palette::{Hsl, IntoColor, Srgb};

const DECAL_DARKEN_AMOUNT: f32 = 0.20;

// ============================================================================
// DecalConfig
// ============================================================================

/// Configuration for decal imprint.
///
/// A decal is a **monochrome SVG** rendered at the center of the icon, filled
/// with a color derived from the underlying pixels (slightly darkened). All
/// fills and strokes in the SVG are replaced with this computed color.
///
/// For full-color SVGs or emojis, use [`SvgOverlayConfig`] instead.
///
/// # Consumed Properties
///
/// - [`DominantColor`]: If set by an upstream layer, uses this color for the decal.
///   Otherwise, derives the color from the [`SurfaceColor`] in the render context.
#[derive(Debug, Clone)]
pub struct DecalConfig {
    /// The SVG source (should be a monochrome/single-color SVG).
    pub source: SvgSource,

    /// Scale factor relative to the icon's content bounds (0.0-1.0).
    pub scale: f32,
}

impl DecalConfig {
    /// Creates a new decal config from an SVG string.
    ///
    /// Decals are intended for monochrome SVGs that will be uniformly
    /// colored based on the icon's dominant color. For full-color SVGs
    /// or emojis, use [`SvgOverlayConfig`] instead.
    ///
    /// The scale is clamped to 0.0-1.0.
    pub fn new(svg: impl Into<String>, scale: f32) -> Self {
        Self {
            source: SvgSource::Raw(svg.into()),
            scale: scale.clamp(0.0, 1.0),
        }
    }

    /// Creates a decal config from an existing [`SvgSource`].
    ///
    /// This is primarily for internal use when deserializing profiles.
    /// Prefer [`DecalConfig::new`] for normal usage.
    pub(crate) fn from_source(source: SvgSource, scale: f32) -> Self {
        Self {
            source,
            scale: scale.clamp(0.0, 1.0),
        }
    }
}

impl LayerConfig for DecalConfig {
    fn differs_from(&self, other: &Self) -> bool {
        self.source != other.source || (self.scale - other.scale).abs() > 0.0001
    }
}

impl LayerEffect for DecalConfig {
    /// Decal depends on the HSL mutation layer (consumes DominantColor).
    fn dependencies(versions: &LayerVersions) -> DependencyVersion {
        DependencyVersion::from_version(versions.hsl)
    }

    fn transform(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
        // Get dominant color from upstream layer, or derive from surface color
        let dominant_color = ctx
            .get::<DominantColor>()
            .map(|c| c.as_tuple())
            .unwrap_or_else(|| {
                // No upstream DominantColor — convert surface color to RGB
                let sc = ctx.get::<SurfaceColor>().expect("SurfaceColor must be set in RenderContext");
                let hsl = Hsl::new(sc.hue, sc.saturation, sc.lightness);
                let rgb: Srgb = hsl.into_color();
                (
                    (rgb.red * 255.0).round() as u8,
                    (rgb.green * 255.0).round() as u8,
                    (rgb.blue * 255.0).round() as u8,
                    255,
                )
            });

        let darkened = darken_color(dominant_color, DECAL_DARKEN_AMOUNT);

        // Calculate decal size based on content bounds
        let bounds = ctx.image.content_bounds;
        let min_dim = bounds.width.min(bounds.height) as f32;
        let decal_size = (min_dim * self.scale) as u32;

        if decal_size == 0 {
            return Ok(());
        }

        // Render the SVG with the darkened color
        let decal_img = render_source_with_color(&self.source, decal_size, Some(darkened))?;

        // Calculate centered position within content bounds
        let center_x = bounds.x as i32 + (bounds.width as i32 - decal_img.width() as i32) / 2;
        let center_y = bounds.y as i32 + (bounds.height as i32 - decal_img.height() as i32) / 2;

        // Composite the decal onto the image
        composite_over(&mut ctx.image.data, &decal_img, center_x, center_y);

        // Update the IconImage with the modified data
        ctx.image = IconImage::new(ctx.image.data.clone(), ctx.image.scale, ctx.image.content_bounds);
        Ok(())
    }
}

// ============================================================================
// Color Utilities
// ============================================================================

/// Darkens an RGBA color by reducing its lightness.
pub fn darken_color(color: (u8, u8, u8, u8), amount: f32) -> (u8, u8, u8, u8) {
    let (r, g, b, a) = color;
    let rgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let mut hsl: Hsl = rgb.into_color();
    hsl.lightness = (hsl.lightness - amount).max(0.0);
    let darkened: Srgb = hsl.into_color();
    (
        (darkened.red * 255.0).round() as u8,
        (darkened.green * 255.0).round() as u8,
        (darkened.blue * 255.0).round() as u8,
        a,
    )
}
