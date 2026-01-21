//! Icon customization engine with layered transformations.

use crate::icon::{IconImage, IconSet};
use crate::layer::{DecalConfig, HueRotationConfig, LayerPipeline, SvgOverlayConfig};
use crate::profile::{
    CustomizationProfile, DecalSettings, HueRotationSettings, OverlaySettings,
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
/// 1. **Hue Rotation** (`pipeline.hue`) - Shifts the hue of all pixels
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
/// use folco_renderer::{IconCustomizer, IconSet, HueRotationConfig, DecalConfig};
///
/// let base_icons = IconSet::new();
/// let mut customizer = IconCustomizer::new(base_icons);
///
/// // Configure layers directly
/// customizer.pipeline.hue.set_config(Some(HueRotationConfig::new(180.0)));
/// customizer.pipeline.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
///
/// // Toggle layers without losing config
/// customizer.pipeline.hue.set_enabled(false);
///
/// // Render
/// let output = customizer.render_all();
/// ```
pub struct IconCustomizer {
    /// The original system folder icon set (never modified).
    base_icons: IconSet,

    /// The layer pipeline. Access layers directly to configure them.
    ///
    /// See [`LayerPipeline`] for the dependency graph and available layers.
    pub pipeline: LayerPipeline,
}

impl IconCustomizer {
    /// Creates a new customizer with the given base icon set.
    pub fn new(base_icons: IconSet) -> Self {
        Self {
            base_icons,
            pipeline: LayerPipeline::default(),
        }
    }

    /// Returns a reference to the base icon set.
    pub fn base_icons(&self) -> &IconSet {
        &self.base_icons
    }

    /// Renders a single icon at the specified logical size.
    ///
    /// Returns the closest matching size from the base icon set,
    /// with all enabled customizations applied. Returns `None` if
    /// the base icon set is empty.
    pub fn render(&mut self, logical_size: u32) -> Option<IconImage> {
        let base = self.base_icons.find_by_logical_size(logical_size)?.clone();
        Some(self.pipeline.render(&base))
    }

    /// Renders all sizes in the base icon set with customizations applied.
    ///
    /// Returns a new `IconSet` containing the rendered images.
    pub fn render_all(&mut self) -> IconSet {
        let base_images: Vec<_> = self.base_icons.iter().cloned().collect();
        let rendered: Vec<_> = base_images
            .iter()
            .map(|base| self.pipeline.render(base))
            .collect();
        IconSet::from_images(rendered)
    }

    /// Clears all layer caches. Useful for freeing memory.
    pub fn clear_cache(&mut self) {
        self.pipeline.invalidate_all();
    }
}

impl Configurable for IconCustomizer {
    /// Applies a profile's settings to this customizer.
    ///
    /// This sets the configuration and enabled state for each layer.
    ///
    /// # Example
    ///
    /// ```
    /// use folco_renderer::{IconCustomizer, IconSet, Configurable, CustomizationProfile, HueRotationSettings};
    ///
    /// let mut customizer = IconCustomizer::new(IconSet::new());
    /// let profile = CustomizationProfile::new()
    ///     .with_hue_rotation(HueRotationSettings { degrees: 90.0, enabled: true });
    ///
    /// customizer.apply_profile(&profile);
    /// ```
    fn apply_profile(&mut self, profile: &CustomizationProfile) {
        // Hue rotation
        if let Some(ref settings) = profile.hue_rotation {
            self.pipeline
                .hue
                .set_config(Some(HueRotationConfig::new(settings.degrees)));
            self.pipeline.hue.set_enabled(settings.enabled);
        } else {
            self.pipeline.hue.set_config(None);
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
    /// # Example
    ///
    /// ```
    /// use folco_renderer::{IconCustomizer, IconSet, Configurable, HueRotationConfig};
    ///
    /// let mut customizer = IconCustomizer::new(IconSet::new());
    /// customizer.pipeline.hue.set_config(Some(HueRotationConfig::new(45.0)));
    ///
    /// let profile = customizer.export_profile();
    /// let json = profile.to_json().unwrap();
    /// ```
    fn export_profile(&self) -> CustomizationProfile {
        let hue_rotation = self.pipeline.hue.config().map(|c| HueRotationSettings {
            degrees: c.degrees,
            enabled: self.pipeline.hue.is_enabled(),
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
            hue_rotation,
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
    use crate::layer::{DecalConfig, HueRotationConfig, Layer, OverlayPosition, SvgOverlayConfig};
    use image::RgbaImage;

    fn create_test_icon_set() -> IconSet {
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

        set
    }

    #[test]
    fn customizer_creation() {
        let icons = create_test_icon_set();
        let customizer = IconCustomizer::new(icons);

        assert!(customizer.pipeline.hue.config().is_none());
        assert!(customizer.pipeline.decal.config().is_none());
        assert!(customizer.pipeline.overlay.config().is_none());
    }

    #[test]
    fn hue_rotation_setting() {
        let icons = create_test_icon_set();
        let mut customizer = IconCustomizer::new(icons);

        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(180.0)));
        assert_eq!(customizer.pipeline.hue.config().unwrap().degrees, 180.0);

        // Test normalization
        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(450.0)));
        assert_eq!(customizer.pipeline.hue.config().unwrap().degrees, 90.0);

        customizer.pipeline.hue.set_config(None);
        assert!(customizer.pipeline.hue.config().is_none());
    }

    #[test]
    fn render_without_customizations() {
        let icons = create_test_icon_set();
        let mut customizer = IconCustomizer::new(icons);

        let rendered = customizer.render(16).unwrap();
        assert_eq!(rendered.dimensions().width, 16);

        // Verify the image is unchanged
        let pixel = rendered.data.get_pixel(0, 0);
        assert_eq!(pixel.0, [255, 0, 0, 255]);
    }

    #[test]
    fn render_all_sizes() {
        let icons = create_test_icon_set();
        let mut customizer = IconCustomizer::new(icons);

        let result = customizer.render_all();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn hue_rotation_applied() {
        let icons = create_test_icon_set();
        let mut customizer = IconCustomizer::new(icons);

        // Rotate red (hue 0) by 120° -> should become greenish
        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(120.0)));

        let rendered = customizer.render(16).unwrap();
        let pixel = rendered.data.get_pixel(0, 0);

        // Green channel should be dominant after rotation
        assert!(
            pixel[1] > pixel[0],
            "Green should be > Red after 120° rotation"
        );
        assert!(
            pixel[1] > pixel[2],
            "Green should be > Blue after 120° rotation"
        );
    }

    #[test]
    fn cache_invalidation_on_hue_change() {
        let icons = create_test_icon_set();
        let mut customizer = IconCustomizer::new(icons);

        // First render
        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(60.0)));
        let first = customizer.render(16).unwrap();

        // Change hue and render again
        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(180.0)));
        let second = customizer.render(16).unwrap();

        // Results should be different
        let p1 = first.data.get_pixel(0, 0);
        let p2 = second.data.get_pixel(0, 0);
        assert_ne!(
            p1.0, p2.0,
            "Different hue rotations should produce different results"
        );
    }

    #[test]
    fn cache_reuse_same_config() {
        let icons = create_test_icon_set();
        let mut customizer = IconCustomizer::new(icons);

        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(90.0)));

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
    fn hue_rotation_toggle() {
        let icons = create_test_icon_set();
        let mut customizer = IconCustomizer::new(icons);

        // Set hue rotation
        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(120.0)));
        assert!(customizer.pipeline.hue.is_enabled());
        let rotated = customizer.render(16).unwrap();
        let rotated_pixel = rotated.data.get_pixel(0, 0).0;

        // Disable - should render as original
        customizer.pipeline.hue.set_enabled(false);
        assert!(!customizer.pipeline.hue.is_enabled());
        // Config preserved!
        assert_eq!(customizer.pipeline.hue.config().unwrap().degrees, 120.0);
        let disabled = customizer.render(16).unwrap();
        assert_eq!(disabled.data.get_pixel(0, 0).0, [255, 0, 0, 255]); // Original red

        // Re-enable - should render rotated again
        customizer.pipeline.hue.set_enabled(true);
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
    fn decal_uses_hue_rotated_dominant_color() {
        use crate::layer::hue_rotation::sample_dominant_color;
        use crate::layer::{DominantColor, LayerEffect, RenderContext};

        // Create a solid red image
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let red_icon = IconImage::new_full_content(red_img, 1.0);

        // Sample dominant color from red base (for comparison)
        let base_color = sample_dominant_color(&red_icon);
        assert_eq!(base_color, (255, 0, 0, 255), "Base should be red");

        // Apply hue rotation (120° rotates red -> green)
        let hue_config = HueRotationConfig::new(120.0);
        let mut ctx = RenderContext::new(red_icon.clone());
        hue_config.transform(&mut ctx);
        hue_config.emit(&mut ctx);

        // Verify hue rotation emitted DominantColor
        let emitted = ctx.get::<DominantColor>();
        assert!(emitted.is_some(), "Hue rotation should emit DominantColor");

        let emitted_color = emitted.unwrap().as_tuple();
        // After 120° rotation, red should become green-ish
        assert!(
            emitted_color.1 > emitted_color.0,
            "Emitted color should have more green than red after 120° rotation"
        );

        // Now apply decal - it should use the emitted color, not re-sample
        let decal_config = DecalConfig::new("<svg></svg>", 0.5);
        decal_config.transform(&mut ctx);
        // Decal doesn't emit, so no emit() call needed

        // The DominantColor property should still be the rotated green
        // (Decal consumes but doesn't overwrite)
        let color_after_decal = ctx.get::<DominantColor>().unwrap().as_tuple();
        assert_eq!(
            color_after_decal, emitted_color,
            "Decal should consume but not overwrite DominantColor"
        );
    }

    #[test]
    fn decal_samples_base_when_hue_disabled() {
        use crate::layer::{CacheKey, DominantColor, LayerVersions, RenderContext};

        // Create a solid blue image
        let mut blue_img = RgbaImage::new(16, 16);
        for pixel in blue_img.pixels_mut() {
            pixel.0 = [0, 0, 255, 255];
        }
        let blue_icon = IconImage::new_full_content(blue_img, 1.0);

        // Set up layers: hue has config but is DISABLED
        let mut hue_layer: Layer<HueRotationConfig> = Layer::default();
        hue_layer.set_config(Some(HueRotationConfig::new(120.0)));
        hue_layer.set_enabled(false); // Disabled!

        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set_config(Some(DecalConfig::new("<svg></svg>", 0.5)));

        // Create context and apply through Layer::apply (not LayerEffect::apply)
        let mut ctx = RenderContext::new(blue_icon.clone());
        let key = CacheKey::from_icon(&blue_icon);
        let versions = LayerVersions {
            hue: hue_layer.version(),
            decal: decal_layer.version(),
            overlay: 0,
        };

        // Apply hue layer (should skip because disabled)
        hue_layer.apply(&mut ctx, key, &versions);

        // Verify no DominantColor was emitted (because hue was skipped)
        assert!(
            ctx.get::<DominantColor>().is_none(),
            "No DominantColor should exist when hue layer is disabled"
        );

        // Image should be unchanged (still blue)
        assert_eq!(
            ctx.image.data.get_pixel(0, 0).0,
            [0, 0, 255, 255],
            "Image should be unchanged when hue is disabled"
        );

        // Apply decal - it should fall back to sampling ctx.image (the base blue)
        decal_layer.apply(&mut ctx, key, &versions);

        // Image still blue (decal is rendered but our test SVG is tiny/empty)
        assert_eq!(
            ctx.image.data.get_pixel(0, 0).0,
            [0, 0, 255, 255],
            "Image should still be blue"
        );
    }

    #[test]
    fn disabled_hue_layer_version_change_invalidates_decal_cache() {
        use crate::layer::{CacheKey, DominantColor, LayerVersions, RenderContext};

        // Create red and blue test icons
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let red_icon = IconImage::new_full_content(red_img, 1.0);
        let key = CacheKey::from_icon(&red_icon);

        // Set up layers
        let mut hue_layer: Layer<HueRotationConfig> = Layer::default();
        hue_layer.set_config(Some(HueRotationConfig::new(120.0)));
        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set_config(Some(DecalConfig::new("<svg></svg>", 0.5)));

        // First render: hue enabled
        let versions_v1 = LayerVersions {
            hue: hue_layer.version(),
            decal: decal_layer.version(),
            overlay: 0,
        };
        let mut ctx1 = RenderContext::new(red_icon.clone());
        hue_layer.apply(&mut ctx1, key, &versions_v1);
        decal_layer.apply(&mut ctx1, key, &versions_v1);

        // Hue should have emitted DominantColor (green-ish after 120° rotation)
        let emitted_with_hue = ctx1.get::<DominantColor>().unwrap().as_tuple();
        assert!(
            emitted_with_hue.1 > emitted_with_hue.0,
            "With hue enabled, emitted color should be green-ish"
        );

        // Now disable hue - version should change
        let old_version = hue_layer.version();
        hue_layer.set_enabled(false);
        let new_version = hue_layer.version();
        assert_ne!(
            old_version, new_version,
            "Disabling layer should change its version"
        );

        // Second render: hue disabled
        let versions_v2 = LayerVersions {
            hue: hue_layer.version(), // New version!
            decal: decal_layer.version(),
            overlay: 0,
        };
        let mut ctx2 = RenderContext::new(red_icon.clone());
        hue_layer.apply(&mut ctx2, key, &versions_v2);
        decal_layer.apply(&mut ctx2, key, &versions_v2);

        // No DominantColor should be emitted (hue was skipped)
        assert!(
            ctx2.get::<DominantColor>().is_none(),
            "With hue disabled, no DominantColor should be emitted"
        );

        // Image should be original red (not rotated)
        assert_eq!(
            ctx2.image.data.get_pixel(0, 0).0,
            [255, 0, 0, 255],
            "With hue disabled, image should be original red"
        );
    }

    #[test]
    fn pipeline_property_flow_with_hue_toggle() {
        // Integration test: verify the full pipeline handles hue toggle correctly
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let mut icons = IconSet::new();
        icons.add_image(IconImage::new_full_content(red_img, 1.0));

        let mut customizer = IconCustomizer::new(icons);

        // Enable both hue rotation and decal
        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(120.0)));
        customizer
            .pipeline
            .decal
            .set_config(Some(DecalConfig::new("<svg></svg>", 0.5)));

        // Render with hue enabled
        let with_hue = customizer.render(16).unwrap();
        let hue_pixel = with_hue.data.get_pixel(0, 0).0;

        // Disable hue but keep decal
        customizer.pipeline.hue.set_enabled(false);
        let without_hue = customizer.render(16).unwrap();
        let no_hue_pixel = without_hue.data.get_pixel(0, 0).0;

        // With hue: image should be rotated (more green than red)
        assert!(
            hue_pixel[1] > hue_pixel[0],
            "With hue enabled, green should dominate"
        );

        // Without hue: image should be original red
        // (decal is rendered but doesn't affect the corner pixel we're testing)
        assert_eq!(
            no_hue_pixel,
            [255, 0, 0, 255],
            "With hue disabled, should be original red"
        );

        // Re-enable hue - should go back to rotated
        customizer.pipeline.hue.set_enabled(true);
        let re_enabled = customizer.render(16).unwrap();
        assert_eq!(
            re_enabled.data.get_pixel(0, 0).0,
            hue_pixel,
            "Re-enabling hue should restore rotated result"
        );
    }
}
