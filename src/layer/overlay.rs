//! SVG overlay layer configuration and application.

use super::svg::{composite_over, render_source, SvgSource};
use super::{DependencyVersion, LayerConfig, LayerEffect, LayerVersions, RenderContext};
use crate::icon::IconImage;

// ============================================================================
// OverlayPosition
// ============================================================================

/// Position for SVG overlay placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPosition {
    /// Bottom-left corner of content bounds.
    BottomLeft,
    /// Bottom-right corner of content bounds.
    BottomRight,
    /// Top-left corner of content bounds.
    TopLeft,
    /// Top-right corner of content bounds.
    TopRight,
    /// Centered within content bounds.
    Center,
}

// ============================================================================
// SvgOverlayConfig
// ============================================================================

/// Configuration for SVG overlay.
///
/// An overlay is rendered on top of all other layers at a specified position.
///
/// # SVG Sources
///
/// The overlay accepts any [`SvgSource`], which can be:
/// - Raw SVG markup via [`SvgSource::from_svg()`]
/// - An emoji character via [`SvgSource::from_emoji()`] (requires `twemoji` feature)
#[derive(Debug, Clone)]
pub struct SvgOverlayConfig {
    /// The SVG source.
    pub source: SvgSource,

    /// Position within the icon's content bounds.
    pub position: OverlayPosition,

    /// Scale factor relative to the icon's content bounds (0.0-1.0).
    pub scale: f32,
}

impl SvgOverlayConfig {
    /// Creates a new overlay config from any SVG source.
    ///
    /// The scale is clamped to 0.0-1.0.
    pub fn new(source: impl Into<SvgSource>, position: OverlayPosition, scale: f32) -> Self {
        Self {
            source: source.into(),
            position,
            scale: scale.clamp(0.0, 1.0),
        }
    }

    /// Creates a new overlay config from an emoji.
    ///
    /// Returns `None` if the emoji is not supported by twemoji_assets.
    /// Only available when the `twemoji` feature is enabled.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji(emoji: &str, position: OverlayPosition, scale: f32) -> Option<Self> {
        Some(Self {
            source: SvgSource::from_emoji(emoji)?,
            position,
            scale: scale.clamp(0.0, 1.0),
        })
    }
}

impl LayerConfig for SvgOverlayConfig {
    fn differs_from(&self, other: &Self) -> bool {
        self.source != other.source
            || self.position != other.position
            || (self.scale - other.scale).abs() > 0.0001
    }
}

impl LayerEffect for SvgOverlayConfig {
    /// Overlay has no upstream dependencies (applied last, on top).
    fn dependencies(_versions: &LayerVersions) -> DependencyVersion {
        DependencyVersion::NONE
    }

    fn transform(&self, ctx: &mut RenderContext) {
        // Calculate overlay size based on content bounds
        let bounds = ctx.image.content_bounds;
        let min_dim = bounds.width.min(bounds.height) as f32;
        let overlay_size = (min_dim * self.scale) as u32;

        if overlay_size == 0 {
            return;
        }

        // Render the SVG
        let Some(overlay_img) = render_source(&self.source, overlay_size) else {
            return;
        };

        // Calculate position based on the position setting
        let (x, y) = self.calculate_position(&bounds, overlay_img.width(), overlay_img.height());

        // Composite the overlay onto the image
        composite_over(&mut ctx.image.data, &overlay_img, x, y);

        // Update the IconImage with the modified data
        ctx.image = IconImage::new(ctx.image.data.clone(), ctx.image.scale, ctx.image.content_bounds);
    }
}

impl SvgOverlayConfig {
    /// Calculates the (x, y) position for the overlay based on the position setting.
    fn calculate_position(
        &self,
        bounds: &crate::icon::RectPx,
        overlay_width: u32,
        overlay_height: u32,
    ) -> (i32, i32) {
        let bx = bounds.x as i32;
        let by = bounds.y as i32;
        let bw = bounds.width as i32;
        let bh = bounds.height as i32;
        let ow = overlay_width as i32;
        let oh = overlay_height as i32;

        match self.position {
            OverlayPosition::TopLeft => (bx, by),
            OverlayPosition::TopRight => (bx + bw - ow, by),
            OverlayPosition::BottomLeft => (bx, by + bh - oh),
            OverlayPosition::BottomRight => (bx + bw - ow, by + bh - oh),
            OverlayPosition::Center => (bx + (bw - ow) / 2, by + (bh - oh) / 2),
        }
    }
}
