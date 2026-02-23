//! SVG overlay layer — configuration and rendering.

use super::svg::{composite_over, render_source, SvgSource};
use super::{CacheKey, CachedOutput, DependencyVersion, Layer, LayerConfig, LayerVersions, RenderContext};
use crate::error::RenderError;
use image::RgbaImage;

// ============================================================================
// OverlayPosition
// ============================================================================

/// Position for SVG overlay placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
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

/// Configuration for SVG overlay — pure data.
///
/// Stores the SVG source, position, and scale. Rendering logic
/// lives on [`Layer<SvgOverlayConfig>`].
///
/// # SVG Sources
///
/// Accepts any [`SvgSource`]:
/// - Raw SVG markup via [`SvgSource::from_svg()`]
/// - An emoji character via [`SvgSource::from_emoji()`] (requires `twemoji` feature)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
    /// Returns an error if the emoji is not supported by twemoji_assets.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji(emoji: &str, position: OverlayPosition, scale: f32) -> Result<Self, RenderError> {
        Ok(Self {
            source: SvgSource::from_emoji(emoji)?,
            position,
            scale: scale.clamp(0.0, 1.0),
        })
    }

    /// Creates a new overlay config from an emoji name (e.g., "duck").
    ///
    /// Returns an error if the name is not recognized by twemoji_assets.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji_name(name: &str, position: OverlayPosition, scale: f32) -> Result<Self, RenderError> {
        Ok(Self {
            source: SvgSource::from_emoji_name(name)?,
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

// ============================================================================
// Layer Rendering
// ============================================================================

impl Layer<SvgOverlayConfig> {
    /// Render this overlay layer, returning a tile for compositing.
    ///
    /// Returns `None` if inactive. The tile is a transparent canvas with
    /// the SVG rendered at the configured position.
    pub fn render_tile(
        &mut self,
        ctx: &mut RenderContext,
        key: CacheKey,
        _versions: &LayerVersions,
    ) -> Result<Option<RgbaImage>, RenderError> {
        if !self.is_active() {
            return Ok(None);
        }

        let deps = DependencyVersion::NONE; // No upstream dependencies

        if let Some(CachedOutput::Tile(tile)) = self.get_cached(key, deps) {
            return Ok(Some(tile.clone()));
        }

        let config = self.config().unwrap();
        let tile = render_overlay(config, ctx)?;

        self.store(key, CachedOutput::Tile(tile.clone()), deps);
        Ok(Some(tile))
    }
}

/// Renders an overlay SVG onto a transparent tile at the configured position.
fn render_overlay(
    config: &SvgOverlayConfig,
    ctx: &RenderContext,
) -> Result<RgbaImage, RenderError> {
    let bounds = ctx.image.content_bounds;
    let min_dim = bounds.width.min(bounds.height) as f32;
    let overlay_size = (min_dim * config.scale) as u32;

    let width = ctx.image.data.width();
    let height = ctx.image.data.height();
    let mut tile = RgbaImage::new(width, height);

    if overlay_size == 0 {
        return Ok(tile);
    }

    let overlay_img = render_source(&config.source, overlay_size)?;

    let (x, y) = calculate_position(config.position, &bounds, overlay_img.width(), overlay_img.height());

    composite_over(&mut tile, &overlay_img, x, y);

    Ok(tile)
}

/// Calculates the (x, y) position for the overlay based on position setting and bounds.
fn calculate_position(
    position: OverlayPosition,
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

    match position {
        OverlayPosition::TopLeft => (bx, by),
        OverlayPosition::TopRight => (bx + bw - ow, by),
        OverlayPosition::BottomLeft => (bx, by + bh - oh),
        OverlayPosition::BottomRight => (bx + bw - ow, by + bh - oh),
        OverlayPosition::Center => (bx + (bw - ow) / 2, by + (bh - oh) / 2),
    }
}
