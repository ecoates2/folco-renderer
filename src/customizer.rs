//! Icon customization engine with layered transformations.

use crate::icon::{IconBase, IconImage, IconSet, SurfaceColor};
use crate::layer::{DecalConfig, HslMutationConfig, LayerPipeline, SvgOverlayConfig};
use crate::error::RenderError;
use crate::profile::{
    CustomizationProfile, DecalSettings, HslMutationSettings, OverlaySettings,
};

// ============================================================================
// Configurable Trait
// ============================================================================

/// Trait for types that can be configured from a [`CustomizationProfile`].
pub trait Configurable {
    /// Applies a profile's settings to this instance.
    fn apply_profile(&mut self, profile: &CustomizationProfile);

    /// Exports the current settings as a profile.
    fn export_profile(&self) -> CustomizationProfile;
}

// ============================================================================
// IconCustomizer
// ============================================================================

/// Main icon customization engine.
///
/// `IconCustomizer` holds a base icon set and applies a pipeline of
/// customization layers. Access layers directly through the [`pipeline`](Self::pipeline)
/// field to configure them.
///
/// # Layer Pipeline
///
/// 1. **HSL Mutation** (`pipeline.hsl`) - Adjusts hue, saturation, and lightness
/// 2. **Decal Imprint** (`pipeline.decal`) - Renders an SVG at the center
/// 3. **SVG Overlay** (`pipeline.overlay`) - Renders an SVG at a corner position
///
/// Each layer implements [`LayerEffect`](crate::layer::LayerEffect), which means it knows:
/// - How to render itself
/// - What properties to emit for downstream layers (e.g., dominant color)
/// - What properties to consume from upstream layers
///
/// # Example
///
/// ```
/// use folco_renderer::{IconCustomizer, IconBase, IconSet, HslMutationConfig, DecalConfig, SurfaceColor};
///
/// let surface = SurfaceColor::new(44.0, 1.0, 0.72);
/// let base = IconBase::new(IconSet::new(), surface);
/// let mut customizer = IconCustomizer::new(base);
///
/// // Configure layers directly
/// customizer.pipeline.hsl.set_config(Some(HslMutationConfig::new(&surface, 200.0, 0.8, 0.5)));
/// customizer.pipeline.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
///
/// // Toggle layers without losing config
/// customizer.pipeline.hsl.set_enabled(false);
///
/// // Render
/// let output = customizer.render_all();
/// ```
pub struct IconCustomizer {
    /// The original system folder icon set (never modified).
    base_icons: IconSet,

    /// The HSL color of the icon's content surface.
    ///
    /// Used to compute deltas when applying a profile with target HSL colors.
    surface_color: SurfaceColor,

    /// The layer pipeline. Access layers directly to configure them.
    ///
    /// See [`LayerPipeline`] for the dependency graph and available layers.
    pub pipeline: LayerPipeline,
}

impl IconCustomizer {
    /// Creates a new customizer with the given base icon set and surface color.
    pub fn new(base: IconBase) -> Self {
        Self {
            base_icons: base.icons,
            surface_color: base.surface_color,
            pipeline: LayerPipeline::default(),
        }
    }

    /// Returns a reference to the base icon set.
    pub fn base_icons(&self) -> &IconSet {
        &self.base_icons
    }

    /// Returns the surface color used for HSL target→delta computation.
    pub fn surface_color(&self) -> &SurfaceColor {
        &self.surface_color
    }

    /// Renders a single icon at the specified logical size.
    ///
    /// Returns the closest matching size from the base icon set,
    /// with all enabled customizations applied.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::NoBaseIcon`] if no base icon matches the size,
    /// or a render error if a layer fails (e.g., invalid SVG or emoji).
    pub fn render(&mut self, logical_size: u32) -> Result<IconImage, RenderError> {
        let base = self
            .base_icons
            .find_by_logical_size(logical_size)
            .ok_or(RenderError::NoBaseIcon { logical_size })?
            .clone();
        self.pipeline.render(&base, &self.surface_color)
    }

    /// Renders all sizes in the base icon set with customizations applied.
    ///
    /// Returns a new `IconSet` containing the rendered images.
    ///
    /// # Errors
    ///
    /// Returns a render error if any layer fails.
    pub fn render_all(&mut self) -> Result<IconSet, RenderError> {
        let base_images: Vec<_> = self.base_icons.iter().cloned().collect();
        let mut rendered = Vec::with_capacity(base_images.len());
        for base in &base_images {
            rendered.push(self.pipeline.render(base, &self.surface_color)?);
        }
        Ok(IconSet::from_images(rendered))
    }

    /// Clears all layer caches. Useful for freeing memory.
    pub fn clear_cache(&mut self) {
        self.pipeline.invalidate_all();
    }
}

impl Configurable for IconCustomizer {
    /// Applies a profile's settings to this customizer.
    ///
    /// HSL mutation settings are expressed as target colors; the customizer
    /// computes the necessary deltas from the stored surface color.
    ///
    /// # Example
    ///
    /// ```
    /// use folco_renderer::{IconCustomizer, IconBase, IconSet, SurfaceColor, Configurable, CustomizationProfile, HslMutationSettings};
    ///
    /// let surface = SurfaceColor::new(44.0, 1.0, 0.72);
    /// let mut customizer = IconCustomizer::new(IconBase::new(IconSet::new(), surface));
    /// let profile = CustomizationProfile::new()
    ///     .with_hsl_mutation(HslMutationSettings {
    ///         target_hue: 200.0, target_saturation: 0.8, target_lightness: 0.5, enabled: true,
    ///     });
    ///
    /// customizer.apply_profile(&profile);
    /// ```
    fn apply_profile(&mut self, profile: &CustomizationProfile) {
        // HSL mutation — convert target color to deltas via surface color
        if let Some(ref settings) = profile.hsl_mutation {
            self.pipeline
                .hsl
                .set_config(Some(HslMutationConfig::new(
                    &self.surface_color,
                    settings.target_hue,
                    settings.target_saturation,
                    settings.target_lightness,
                )));
            self.pipeline.hsl.set_enabled(settings.enabled);
        } else {
            self.pipeline.hsl.set_config(None);
        }

        // Decal
        if let Some(ref settings) = profile.decal {
            let source: crate::layer::SvgSource = settings.source.clone().into();
            self.pipeline
                .decal
                .set_config(Some(DecalConfig::from_source(source, settings.scale)));
            self.pipeline.decal.set_enabled(settings.enabled);
        } else {
            self.pipeline.decal.set_config(None);
        }

        // Overlay
        if let Some(ref settings) = profile.overlay {
            let source: crate::layer::SvgSource = settings.source.clone().into();
            self.pipeline.overlay.set_config(Some(SvgOverlayConfig::new(
                source,
                settings.position.into(),
                settings.scale,
            )));
            self.pipeline.overlay.set_enabled(settings.enabled);
        } else {
            self.pipeline.overlay.set_config(None);
        }
    }

    /// Exports the current customization settings as a profile.
    ///
    /// HSL mutation settings are exported as target colors, reverse-computed
    /// from the internal deltas and the stored surface color.
    ///
    /// # Example
    ///
    /// ```
    /// use folco_renderer::{IconCustomizer, IconBase, IconSet, SurfaceColor, Configurable, HslMutationConfig};
    ///
    /// let surface = SurfaceColor::new(44.0, 1.0, 0.72);
    /// let mut customizer = IconCustomizer::new(IconBase::new(IconSet::new(), surface));
    /// customizer.pipeline.hsl.set_config(Some(HslMutationConfig::new(&surface, 200.0, 0.8, 0.5)));
    ///
    /// let profile = customizer.export_profile();
    /// let json = profile.to_json().unwrap();
    /// ```
    fn export_profile(&self) -> CustomizationProfile {
        let hsl_mutation = self.pipeline.hsl.config().map(|c| HslMutationSettings {
            target_hue: c.target_hue,
            target_saturation: c.target_saturation,
            target_lightness: c.target_lightness,
            enabled: self.pipeline.hsl.is_enabled(),
        });

        let decal = self.pipeline.decal.config().map(|c| DecalSettings {
            source: (&c.source).into(),
            scale: c.scale,
            enabled: self.pipeline.decal.is_enabled(),
        });

        let overlay = self.pipeline.overlay.config().map(|c| OverlaySettings {
            source: (&c.source).into(),
            position: c.position.into(),
            scale: c.scale,
            enabled: self.pipeline.overlay.is_enabled(),
        });

        CustomizationProfile {
            hsl_mutation,
            decal,
            overlay,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::decal::darken_color;
    use crate::layer::{DecalConfig, HslMutationConfig, Layer, OverlayPosition, SvgOverlayConfig};
    use image::RgbaImage;

    const TEST_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"##;

    /// Test surface color — uses the same golden-yellow as Windows folders.
    const TEST_SURFACE: SurfaceColor = SurfaceColor::new(44.0, 1.0, 0.72);

    fn create_test_icon_base() -> IconBase {
        let mut set = IconSet::new();

        // Create a 16x16 red icon
        let mut img16 = RgbaImage::new(16, 16);
        for pixel in img16.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        set.add_image(IconImage::new_full_content(img16, 1.0));

        // Create a 32x32 green icon
        let mut img32 = RgbaImage::new(32, 32);
        for pixel in img32.pixels_mut() {
            pixel.0 = [0, 255, 0, 255];
        }
        set.add_image(IconImage::new_full_content(img32, 1.0));

        IconBase::new(set, TEST_SURFACE)
    }

    #[test]
    fn customizer_creation() {
        let base = create_test_icon_base();
        let customizer = IconCustomizer::new(base);

        assert!(customizer.pipeline.hsl.config().is_none());
        assert!(customizer.pipeline.decal.config().is_none());
        assert!(customizer.pipeline.overlay.config().is_none());
    }

    #[test]
    fn hsl_mutation_setting() {
        let base = create_test_icon_base();
        let mut customizer = IconCustomizer::new(base);

        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 224.0, 0.8, 0.504)));
        assert!((customizer.pipeline.hsl.config().unwrap().target_hue - 224.0).abs() < 0.01);
        assert!((customizer.pipeline.hsl.config().unwrap().target_saturation - 0.8).abs() < 0.01);
        assert!((customizer.pipeline.hsl.config().unwrap().target_lightness - 0.504).abs() < 0.01);

        // Test normalization (450° wraps to 90°)
        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 450.0, 1.0, 0.72)));
        assert!((customizer.pipeline.hsl.config().unwrap().target_hue - 90.0).abs() < 0.01);

        customizer.pipeline.hsl.set_config(None);
        assert!(customizer.pipeline.hsl.config().is_none());
    }

    #[test]
    fn render_without_customizations() {
        let base = create_test_icon_base();
        let mut customizer = IconCustomizer::new(base);

        let rendered = customizer.render(16).unwrap();
        assert_eq!(rendered.dimensions().width, 16);

        // Verify the image is unchanged
        let pixel = rendered.data.get_pixel(0, 0);
        assert_eq!(pixel.0, [255, 0, 0, 255]);
    }

    #[test]
    fn render_all_sizes() {
        let base = create_test_icon_base();
        let mut customizer = IconCustomizer::new(base);

        let result = customizer.render_all().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn hsl_mutation_applied() {
        let base = create_test_icon_base();
        let mut customizer = IconCustomizer::new(base);

        // Rotate red (hue 0) by 120° -> should become greenish
        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 164.0, 1.0, 0.72)));

        let rendered = customizer.render(16).unwrap();
        let pixel = rendered.data.get_pixel(0, 0);

        // Green channel should be dominant after rotation
        assert!(
            pixel[1] > pixel[0],
            "Green should be > Red after 120° hue shift"
        );
        assert!(
            pixel[1] > pixel[2],
            "Green should be > Blue after 120° hue shift"
        );
    }

    #[test]
    fn cache_invalidation_on_hsl_change() {
        let base = create_test_icon_base();
        let mut customizer = IconCustomizer::new(base);

        // First render
        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 104.0, 1.0, 0.72)));
        let first = customizer.render(16).unwrap();

        // Change hue and render again
        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 224.0, 1.0, 0.72)));
        let second = customizer.render(16).unwrap();

        // Results should be different
        let p1 = first.data.get_pixel(0, 0);
        let p2 = second.data.get_pixel(0, 0);
        assert_ne!(
            p1.0, p2.0,
            "Different hue shifts should produce different results"
        );
    }

    #[test]
    fn cache_reuse_same_config() {
        let base = create_test_icon_base();
        let mut customizer = IconCustomizer::new(base);

        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 134.0, 1.0, 0.72)));

        // Render twice with same config
        let first = customizer.render(16).unwrap();
        let second = customizer.render(16).unwrap();

        // Results should be identical (from cache)
        assert_eq!(first.data.get_pixel(0, 0), second.data.get_pixel(0, 0));
    }

    #[test]
    fn layer_generic_set_config() {
        let mut layer: Layer<DecalConfig> = Layer::default();

        // Enabled by default, but no config yet
        assert!(layer.is_enabled());
        assert!(!layer.has_config());
        assert!(!layer.is_active()); // Not active without config
        assert_eq!(layer.version(), 0);

        // Setting config should increment version
        let changed = layer.set_config(Some(DecalConfig::new("svg", 0.5)));
        assert!(changed);
        assert!(layer.has_config());
        assert!(layer.is_active()); // Now active
        assert_eq!(layer.version(), 1);

        // Same config should not increment
        let changed = layer.set_config(Some(DecalConfig::new("svg", 0.5)));
        assert!(!changed);
        assert_eq!(layer.version(), 1);

        // Different config should increment
        let changed = layer.set_config(Some(DecalConfig::new("svg2", 0.5)));
        assert!(changed);
        assert_eq!(layer.version(), 2);
    }

    #[test]
    fn layer_toggle_without_losing_config() {
        let mut layer: Layer<DecalConfig> = Layer::default();

        // Set config
        layer.set_config(Some(DecalConfig::new("my-svg", 0.5)));
        assert!(layer.is_active());
        assert_eq!(layer.version(), 1);

        // Disable - config should be preserved
        let changed = layer.set_enabled(false);
        assert!(changed);
        assert!(!layer.is_active());
        assert!(layer.has_config()); // Config still there!
        assert_eq!(layer.config().unwrap().source, crate::layer::SvgSource::Raw("my-svg".into()));
        assert_eq!(layer.version(), 2);

        // Re-enable - should work with same config
        let changed = layer.set_enabled(true);
        assert!(changed);
        assert!(layer.is_active());
        assert_eq!(layer.config().unwrap().source, crate::layer::SvgSource::Raw("my-svg".into()));
        assert_eq!(layer.version(), 3);
    }

    #[test]
    fn hsl_mutation_toggle() {
        let base = create_test_icon_base();
        let mut customizer = IconCustomizer::new(base);

        // Set HSL mutation
        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 164.0, 1.0, 0.72)));
        assert!(customizer.pipeline.hsl.is_enabled());
        let rotated = customizer.render(16).unwrap();
        let rotated_pixel = rotated.data.get_pixel(0, 0).0;

        // Disable - should render as original
        customizer.pipeline.hsl.set_enabled(false);
        assert!(!customizer.pipeline.hsl.is_enabled());
        // Config preserved!
        assert!((customizer.pipeline.hsl.config().unwrap().target_hue - 164.0).abs() < 0.01);
        let disabled = customizer.render(16).unwrap();
        assert_eq!(disabled.data.get_pixel(0, 0).0, [255, 0, 0, 255]); // Original red

        // Re-enable - should render rotated again
        customizer.pipeline.hsl.set_enabled(true);
        let re_enabled = customizer.render(16).unwrap();
        assert_eq!(re_enabled.data.get_pixel(0, 0).0, rotated_pixel);
    }

    #[test]
    fn darken_color_works() {
        let original = (200, 100, 100, 255);
        let darkened = darken_color(original, 0.2);

        // Darkened color should have lower overall brightness
        let orig_brightness = (original.0 as u32 + original.1 as u32 + original.2 as u32) / 3;
        let dark_brightness = (darkened.0 as u32 + darkened.1 as u32 + darkened.2 as u32) / 3;
        assert!(
            dark_brightness < orig_brightness,
            "Darkened color should be less bright"
        );

        // Alpha should be preserved
        assert_eq!(darkened.3, original.3);
    }

    #[test]
    fn decal_config_creation() {
        let config = DecalConfig::new("<svg></svg>", 0.5);
        assert_eq!(config.source, crate::layer::SvgSource::Raw("<svg></svg>".into()));
        assert_eq!(config.scale, 0.5);

        // Test clamping
        let clamped = DecalConfig::new("svg", 1.5);
        assert_eq!(clamped.scale, 1.0);
    }

    #[test]
    fn overlay_config_creation() {
        let config = SvgOverlayConfig::new("<svg></svg>", OverlayPosition::BottomRight, 0.25);
        assert_eq!(config.position, OverlayPosition::BottomRight);
        assert_eq!(config.scale, 0.25);
    }

    #[test]
    fn decal_uses_hsl_mutated_dominant_color() {
        use crate::layer::{DominantColor, LayerEffect, RenderContext};

        // Create a solid red image
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let red_icon = IconImage::new_full_content(red_img, 1.0);

        // Apply hue shift (120° rotates red -> green)
        let hsl_config = HslMutationConfig::new(&TEST_SURFACE, 164.0, 1.0, 0.72);
        let mut ctx = RenderContext::new(red_icon.clone());
        ctx.set(TEST_SURFACE);
        hsl_config.transform(&mut ctx).unwrap();
        hsl_config.emit(&mut ctx);

        // Verify HSL mutation emitted DominantColor
        let emitted = ctx.get::<DominantColor>();
        assert!(emitted.is_some(), "HSL mutation should emit DominantColor");

        let emitted_color = emitted.unwrap().as_tuple();
        // After 120° hue shift, red should become green-ish
        assert!(
            emitted_color.1 > emitted_color.0,
            "Emitted color should have more green than red after 120° hue shift"
        );

        // Now apply decal - it should use the emitted color, not re-sample
        let decal_config = DecalConfig::new(TEST_SVG, 0.5);
        decal_config.transform(&mut ctx).unwrap();
        // Decal doesn't emit, so no emit() call needed

        // The DominantColor property should still be the shifted green
        // (Decal consumes but doesn't overwrite)
        let color_after_decal = ctx.get::<DominantColor>().unwrap().as_tuple();
        assert_eq!(
            color_after_decal, emitted_color,
            "Decal should consume but not overwrite DominantColor"
        );
    }

    #[test]
    fn decal_samples_base_when_hsl_disabled() {
        use crate::layer::{CacheKey, DominantColor, LayerVersions, RenderContext};

        // Create a solid blue image
        let mut blue_img = RgbaImage::new(16, 16);
        for pixel in blue_img.pixels_mut() {
            pixel.0 = [0, 0, 255, 255];
        }
        let blue_icon = IconImage::new_full_content(blue_img, 1.0);

        // Set up layers: HSL has config but is DISABLED
        let mut hsl_layer: Layer<HslMutationConfig> = Layer::default();
        hsl_layer.set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 164.0, 1.0, 0.72)));
        hsl_layer.set_enabled(false); // Disabled!

        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // Create context and apply through Layer::apply (not LayerEffect::apply)
        let mut ctx = RenderContext::new(blue_icon.clone());
        ctx.set(TEST_SURFACE);
        let key = CacheKey::from_icon(&blue_icon);
        let versions = LayerVersions {
            hsl: hsl_layer.version(),
            decal: decal_layer.version(),
            overlay: 0,
        };

        // Apply HSL layer (should skip because disabled)
        hsl_layer.apply(&mut ctx, key, &versions).unwrap();

        // Verify no DominantColor was emitted (because HSL was skipped)
        assert!(
            ctx.get::<DominantColor>().is_none(),
            "No DominantColor should exist when HSL layer is disabled"
        );

        // Image should be unchanged (still blue)
        assert_eq!(
            ctx.image.data.get_pixel(0, 0).0,
            [0, 0, 255, 255],
            "Image should be unchanged when HSL is disabled"
        );

        // Apply decal - it should fall back to surface color (the golden-yellow)
        decal_layer.apply(&mut ctx, key, &versions).unwrap();

        // Image still blue at corner (decal renders at center, not at (0,0))
        assert_eq!(
            ctx.image.data.get_pixel(0, 0).0,
            [0, 0, 255, 255],
            "Image should still be blue"
        );
    }

    #[test]
    fn disabled_hsl_layer_version_change_invalidates_decal_cache() {
        use crate::layer::{CacheKey, DominantColor, LayerVersions, RenderContext};

        // Create red and blue test icons
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let red_icon = IconImage::new_full_content(red_img, 1.0);
        let key = CacheKey::from_icon(&red_icon);

        // Set up layers
        let mut hsl_layer: Layer<HslMutationConfig> = Layer::default();
        hsl_layer.set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 164.0, 1.0, 0.72)));
        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // First render: HSL enabled
        let versions_v1 = LayerVersions {
            hsl: hsl_layer.version(),
            decal: decal_layer.version(),
            overlay: 0,
        };
        let mut ctx1 = RenderContext::new(red_icon.clone());
        ctx1.set(TEST_SURFACE);
        hsl_layer.apply(&mut ctx1, key, &versions_v1).unwrap();
        decal_layer.apply(&mut ctx1, key, &versions_v1).unwrap();

        // HSL should have emitted DominantColor (green-ish after 120° shift)
        let emitted_with_hsl = ctx1.get::<DominantColor>().unwrap().as_tuple();
        assert!(
            emitted_with_hsl.1 > emitted_with_hsl.0,
            "With HSL enabled, emitted color should be green-ish"
        );

        // Now disable HSL - version should change
        let old_version = hsl_layer.version();
        hsl_layer.set_enabled(false);
        let new_version = hsl_layer.version();
        assert_ne!(
            old_version, new_version,
            "Disabling layer should change its version"
        );

        // Second render: HSL disabled
        let versions_v2 = LayerVersions {
            hsl: hsl_layer.version(), // New version!
            decal: decal_layer.version(),
            overlay: 0,
        };
        let mut ctx2 = RenderContext::new(red_icon.clone());
        ctx2.set(TEST_SURFACE);
        hsl_layer.apply(&mut ctx2, key, &versions_v2).unwrap();
        decal_layer.apply(&mut ctx2, key, &versions_v2).unwrap();

        // No DominantColor should be emitted (HSL was skipped)
        assert!(
            ctx2.get::<DominantColor>().is_none(),
            "With HSL disabled, no DominantColor should be emitted"
        );

        // Image should be original red (not shifted)
        assert_eq!(
            ctx2.image.data.get_pixel(0, 0).0,
            [255, 0, 0, 255],
            "With HSL disabled, image should be original red"
        );
    }

    #[test]
    fn pipeline_property_flow_with_hsl_toggle() {
        // Integration test: verify the full pipeline handles HSL toggle correctly
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let mut icons = IconSet::new();
        icons.add_image(IconImage::new_full_content(red_img, 1.0));

        let mut customizer = IconCustomizer::new(IconBase::new(icons, TEST_SURFACE));

        // Enable both HSL mutation and decal
        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&TEST_SURFACE, 164.0, 1.0, 0.72)));
        customizer
            .pipeline
            .decal
            .set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // Render with HSL enabled
        let with_hsl = customizer.render(16).unwrap();
        let hsl_pixel = with_hsl.data.get_pixel(0, 0).0;

        // Disable HSL but keep decal
        customizer.pipeline.hsl.set_enabled(false);
        let without_hsl = customizer.render(16).unwrap();
        let no_hsl_pixel = without_hsl.data.get_pixel(0, 0).0;

        // With HSL: image should be shifted (more green than red)
        assert!(
            hsl_pixel[1] > hsl_pixel[0],
            "With HSL enabled, green should dominate"
        );

        // Without HSL: image should be original red
        assert_eq!(
            no_hsl_pixel,
            [255, 0, 0, 255],
            "With HSL disabled, should be original red"
        );

        // Re-enable HSL - should go back to shifted
        customizer.pipeline.hsl.set_enabled(true);
        let re_enabled = customizer.render(16).unwrap();
        assert_eq!(
            re_enabled.data.get_pixel(0, 0).0,
            hsl_pixel,
            "Re-enabling HSL should restore shifted result"
        );
    }
}
