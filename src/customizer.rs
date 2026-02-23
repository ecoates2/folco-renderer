//! Icon customization engine with layered transformations.

use crate::icon::{FolderIconBase, IconImage, IconSet, SurfaceColor};
use crate::layer::LayerPipeline;
use crate::error::RenderError;
use crate::profile::CustomizationProfile;

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
// FolderIconCustomizer
// ============================================================================

/// Main icon customization engine.
///
/// `FolderIconCustomizer` holds a base icon set and applies a pipeline of
/// customization layers. Access layers directly through the [`pipeline`](Self::pipeline)
/// field to configure them.
///
/// # Layer Pipeline
///
/// 1. **Color Target** (`pipeline.folder_color_target`) - Recolors to a target RGB color
/// 2. **Decal Imprint** (`pipeline.decal`) - Renders an SVG at the center
/// 3. **SVG Overlay** (`pipeline.overlay`) - Renders an SVG at a corner position
///
/// Each layer's rendering logic lives on its concrete `Layer<Config>` type,
/// keeping config structs as pure data.
///
/// # Example
///
/// ```
/// use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, FolderColorTargetConfig, DecalConfig, SurfaceColor};
///
/// let surface = SurfaceColor::new(255, 217, 112);
/// let base = FolderIconBase::new(IconSet::new(), surface);
/// let mut customizer = FolderIconCustomizer::new(base);
///
/// // Configure layers directly
/// customizer.pipeline.folder_color_target.set_config(Some(FolderColorTargetConfig::new(33, 150, 243)));
/// customizer.pipeline.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
///
/// // Toggle a layer off without losing config
/// customizer.pipeline.folder_color_target.set_enabled(false);
///
/// // Render
/// let output = customizer.render_all();
/// ```
pub struct FolderIconCustomizer {
    /// The original system folder icon set (never modified).
    base_icons: IconSet,

    /// The RGB color of the icon's content surface.
    ///
    /// Used to compute deltas when applying a profile with target RGB colors.
    surface_color: SurfaceColor,

    /// The layer pipeline. Access layers directly to configure them.
    ///
    /// See [`LayerPipeline`] for the dependency graph and available layers.
    pub pipeline: LayerPipeline,
}

impl FolderIconCustomizer {
    /// Creates a new customizer with the given base icon set and surface color.
    pub fn new(base: FolderIconBase) -> Self {
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

    /// Returns the surface color used for color target delta computation.
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

impl Configurable for FolderIconCustomizer {
    /// Applies a profile's settings to this customizer.
    ///
    /// Color target settings are expressed as target RGB colors; the customizer
    /// computes the necessary HSL deltas from the stored surface color internally.
    ///
    /// # Example
    ///
    /// ```
    /// use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, SurfaceColor, Configurable, CustomizationProfile, FolderColorTargetConfig};
    ///
    /// let surface = SurfaceColor::new(255, 217, 112);
    /// let mut customizer = FolderIconCustomizer::new(FolderIconBase::new(IconSet::new(), surface));
    /// let profile = CustomizationProfile::new()
    ///     .with_folder_color_target(FolderColorTargetConfig::new(33, 150, 243));
    ///
    /// customizer.apply_profile(&profile);
    /// ```
    fn apply_profile(&mut self, profile: &CustomizationProfile) {
        self.pipeline.folder_color_target.set_config(profile.folder_color_target.clone());
        self.pipeline.decal.set_config(profile.decal.clone());
        self.pipeline.overlay.set_config(profile.overlay.clone());
    }

    /// Exports the current customization settings as a profile.
    ///
    /// Color target settings are exported as target RGB colors directly
    /// from the stored config.
    ///
    /// # Example
    ///
    /// ```
    /// use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, SurfaceColor, Configurable, FolderColorTargetConfig};
    ///
    /// let surface = SurfaceColor::new(255, 217, 112);
    /// let mut customizer = FolderIconCustomizer::new(FolderIconBase::new(IconSet::new(), surface));
    /// customizer.pipeline.folder_color_target.set_config(Some(FolderColorTargetConfig::new(33, 150, 243)));
    ///
    /// let profile = customizer.export_profile();
    /// let json = profile.to_json().unwrap();
    /// ```
    fn export_profile(&self) -> CustomizationProfile {
        CustomizationProfile {
            folder_color_target: self.pipeline.folder_color_target.config().cloned(),
            decal: self.pipeline.decal.config().cloned(),
            overlay: self.pipeline.overlay.config().cloned(),
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
    use crate::layer::{DecalConfig, FolderColorTargetConfig, Layer, OverlayPosition, SvgOverlayConfig};
    use image::RgbaImage;

    const TEST_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"##;

    /// Test surface color — uses the same golden-yellow as Windows folders.
    const TEST_SURFACE: SurfaceColor = SurfaceColor::new(255, 217, 112);

    fn create_test_icon_base() -> FolderIconBase {
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

        FolderIconBase::new(set, TEST_SURFACE)
    }

    #[test]
    fn customizer_creation() {
        let base = create_test_icon_base();
        let customizer = FolderIconCustomizer::new(base);

        assert!(customizer.pipeline.folder_color_target.config().is_none());
        assert!(customizer.pipeline.decal.config().is_none());
        assert!(customizer.pipeline.overlay.config().is_none());
    }

    #[test]
    fn hsl_mutation_setting() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::new(base);

        customizer
            .pipeline
            .folder_color_target
            .set_config(Some(FolderColorTargetConfig::new(33, 150, 243)));
        assert_eq!(customizer.pipeline.folder_color_target.config().unwrap().target_r, 33);
        assert_eq!(customizer.pipeline.folder_color_target.config().unwrap().target_g, 150);
        assert_eq!(customizer.pipeline.folder_color_target.config().unwrap().target_b, 243);

        customizer.pipeline.folder_color_target.set_config(None);
        assert!(customizer.pipeline.folder_color_target.config().is_none());
    }

    #[test]
    fn render_without_customizations() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::new(base);

        let rendered = customizer.render(16).unwrap();
        assert_eq!(rendered.dimensions().width, 16);

        // Verify the image is unchanged
        let pixel = rendered.data.get_pixel(0, 0);
        assert_eq!(pixel.0, [255, 0, 0, 255]);
    }

    #[test]
    fn render_all_sizes() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::new(base);

        let result = customizer.render_all().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn hsl_mutation_applied() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::new(base);

        // Apply a green-ish target color
        customizer
            .pipeline
            .folder_color_target
            .set_config(Some(FolderColorTargetConfig::new(0, 188, 212)));

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
        let mut customizer = FolderIconCustomizer::new(base);

        // First render
        customizer
            .pipeline
            .folder_color_target
            .set_config(Some(FolderColorTargetConfig::new(76, 175, 80)));
        let first = customizer.render(16).unwrap();

        // Change color and render again
        customizer
            .pipeline
            .folder_color_target
            .set_config(Some(FolderColorTargetConfig::new(33, 150, 243)));
        let second = customizer.render(16).unwrap();

        // Results should be different
        let p1 = first.data.get_pixel(0, 0);
        let p2 = second.data.get_pixel(0, 0);
        assert_ne!(
            p1.0, p2.0,
            "Different target colors should produce different results"
        );
    }

    #[test]
    fn cache_reuse_same_config() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::new(base);

        customizer
            .pipeline
            .folder_color_target
            .set_config(Some(FolderColorTargetConfig::new(76, 175, 80)));

        // Render twice with same config
        let first = customizer.render(16).unwrap();
        let second = customizer.render(16).unwrap();

        // Results should be identical (from cache)
        assert_eq!(first.data.get_pixel(0, 0), second.data.get_pixel(0, 0));
    }

    #[test]
    fn layer_generic_set_config() {
        let mut layer: Layer<DecalConfig> = Layer::default();

        // No config yet
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
        assert!(layer.is_enabled());
        assert_eq!(layer.version(), 1);

        // Disable via toggle — config preserved
        layer.set_enabled(false);
        assert!(!layer.is_active());
        assert!(!layer.is_enabled());
        assert!(layer.has_config()); // Config still present!
        assert_eq!(layer.version(), 2);

        // Re-enable — should be active again
        layer.set_enabled(true);
        assert!(layer.is_active());
        assert!(layer.is_enabled());
        assert_eq!(layer.config().unwrap().source, crate::layer::SvgSource::Raw("my-svg".into()));
        assert_eq!(layer.version(), 3);

        // set_enabled(true) when already enabled should not bump version
        let changed = layer.set_enabled(true);
        assert!(!changed);
        assert_eq!(layer.version(), 3);
    }

    #[test]
    fn layer_config_set_clear_cycle() {
        let mut layer: Layer<DecalConfig> = Layer::default();

        // Set config
        layer.set_config(Some(DecalConfig::new("my-svg", 0.5)));
        assert!(layer.is_active());
        assert_eq!(layer.version(), 1);

        // Clear config (disable)
        let changed = layer.set_config(None);
        assert!(changed);
        assert!(!layer.is_active());
        assert!(!layer.has_config());
        assert_eq!(layer.version(), 2);

        // Re-set config (re-enable)
        let changed = layer.set_config(Some(DecalConfig::new("my-svg", 0.5)));
        assert!(changed);
        assert!(layer.is_active());
        assert_eq!(layer.config().unwrap().source, crate::layer::SvgSource::Raw("my-svg".into()));
        assert_eq!(layer.version(), 3);
    }

    #[test]
    fn hsl_mutation_toggle() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::new(base);

        // Set color target
        customizer
            .pipeline
            .folder_color_target
            .set_config(Some(FolderColorTargetConfig::new(0, 188, 212)));
        assert!(customizer.pipeline.folder_color_target.is_active());
        let rotated = customizer.render(16).unwrap();
        let rotated_pixel = rotated.data.get_pixel(0, 0).0;

        // Disable via toggle (config and cache preserved)
        customizer.pipeline.folder_color_target.set_enabled(false);
        assert!(!customizer.pipeline.folder_color_target.is_active());
        assert!(customizer.pipeline.folder_color_target.has_config()); // Config still present
        let disabled = customizer.render(16).unwrap();
        assert_eq!(disabled.data.get_pixel(0, 0).0, [255, 0, 0, 255]); // Original red

        // Re-enable
        customizer.pipeline.folder_color_target.set_enabled(true);
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
        use crate::layer::folder_color_target::apply_folder_color_target;
        use crate::layer::decal::render_decal;
        use crate::layer::{DominantColor, RenderContext};

        // Create a solid red image
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let red_icon = IconImage::new_full_content(red_img, 1.0);

        // Apply color target (cyan-ish)
        let config = FolderColorTargetConfig::new(0, 188, 212);
        let mut ctx = RenderContext::new(red_icon.clone());
        ctx.set(TEST_SURFACE);
        ctx.image = apply_folder_color_target(&ctx.image, &TEST_SURFACE, &config);
        ctx.set(DominantColor::new(config.target_r, config.target_g, config.target_b, 255));

        // Verify color target emitted DominantColor
        let emitted = ctx.get::<DominantColor>();
        assert!(emitted.is_some(), "Color target should emit DominantColor");

        let emitted_color = emitted.unwrap().as_tuple();
        assert!(
            emitted_color.1 > emitted_color.0,
            "Emitted color should have more green than red for cyan target"
        );

        // Now apply decal - it should use the emitted color, not re-sample
        let decal_config = DecalConfig::new(TEST_SVG, 0.5);
        let _tile = render_decal(&decal_config, &ctx).unwrap();

        // The DominantColor property should still be the target color
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

        // Set up layers: color target NOT configured (disabled)
        let mut ct_layer: Layer<FolderColorTargetConfig> = Layer::default();
        // No config set — layer is inactive

        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // Create context and apply through Layer::apply
        let mut ctx = RenderContext::new(blue_icon.clone());
        ctx.set(TEST_SURFACE);
        let key = CacheKey::from_icon(&blue_icon);
        let versions = LayerVersions {
            folder_color_target: ct_layer.version(),
            decal: decal_layer.version(),
            overlay: 0,
        };

        // Apply color target layer (should skip because no config)
        ct_layer.apply(&mut ctx, key, &versions).unwrap();

        // Verify no DominantColor was emitted (because color target was skipped)
        assert!(
            ctx.get::<DominantColor>().is_none(),
            "No DominantColor should exist when color target layer has no config"
        );

        // Image should be unchanged (still blue)
        assert_eq!(
            ctx.image.data.get_pixel(0, 0).0,
            [0, 0, 255, 255],
            "Image should be unchanged when color target has no config"
        );

        // Apply decal - it should fall back to surface color (the golden-yellow)
        // Decal now returns a tile, not modifying ctx.image directly
        let _tile = decal_layer.render_tile(&mut ctx, key, &versions).unwrap();

        // Image should still be unchanged (decal produces a tile, doesn't composite)
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
        let mut ct_layer: Layer<FolderColorTargetConfig> = Layer::default();
        ct_layer.set_config(Some(FolderColorTargetConfig::new(0, 188, 212)));
        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // First render: color target enabled
        let versions_v1 = LayerVersions {
            folder_color_target: ct_layer.version(),
            decal: decal_layer.version(),
            overlay: 0,
        };
        let mut ctx1 = RenderContext::new(red_icon.clone());
        ctx1.set(TEST_SURFACE);
        ct_layer.apply(&mut ctx1, key, &versions_v1).unwrap();
        decal_layer.render_tile(&mut ctx1, key, &versions_v1).unwrap();

        // Color target should have emitted DominantColor
        let emitted_with_ct = ctx1.get::<DominantColor>().unwrap().as_tuple();
        assert!(
            emitted_with_ct.1 > emitted_with_ct.0,
            "With color target enabled, emitted color should be cyan-ish"
        );

        // Now clear color target config — version should change
        let old_version = ct_layer.version();
        ct_layer.set_config(None);
        let new_version = ct_layer.version();
        assert_ne!(
            old_version, new_version,
            "Clearing config should change its version"
        );

        // Second render: color target not configured
        let versions_v2 = LayerVersions {
            folder_color_target: ct_layer.version(), // New version!
            decal: decal_layer.version(),
            overlay: 0,
        };
        let mut ctx2 = RenderContext::new(red_icon.clone());
        ctx2.set(TEST_SURFACE);
        ct_layer.apply(&mut ctx2, key, &versions_v2).unwrap();
        decal_layer.render_tile(&mut ctx2, key, &versions_v2).unwrap();

        // No DominantColor should be emitted (color target was skipped)
        assert!(
            ctx2.get::<DominantColor>().is_none(),
            "With color target cleared, no DominantColor should be emitted"
        );

        // Image should be original red (not shifted)
        assert_eq!(
            ctx2.image.data.get_pixel(0, 0).0,
            [255, 0, 0, 255],
            "With color target cleared, image should be original red"
        );
    }

    #[test]
    fn pipeline_property_flow_with_hsl_toggle() {
        // Integration test: verify the full pipeline handles color target toggle correctly
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let mut icons = IconSet::new();
        icons.add_image(IconImage::new_full_content(red_img, 1.0));

        let mut customizer = FolderIconCustomizer::new(FolderIconBase::new(icons, TEST_SURFACE));

        // Enable both color target and decal
        let ct_config = FolderColorTargetConfig::new(0, 188, 212);
        customizer
            .pipeline
            .folder_color_target
            .set_config(Some(ct_config.clone()));
        customizer
            .pipeline
            .decal
            .set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // Render with color target enabled
        let with_ct = customizer.render(16).unwrap();
        let ct_pixel = with_ct.data.get_pixel(0, 0).0;

        // Disable color target via toggle, keep decal
        customizer.pipeline.folder_color_target.set_enabled(false);
        let without_ct = customizer.render(16).unwrap();
        let no_ct_pixel = without_ct.data.get_pixel(0, 0).0;

        // With color target: image should be shifted (more green than red)
        assert!(
            ct_pixel[1] > ct_pixel[0],
            "With color target enabled, green should dominate"
        );

        // Without color target: image should be original red
        assert_eq!(
            no_ct_pixel,
            [255, 0, 0, 255],
            "With color target disabled, should be original red"
        );

        // Re-enable color target - should go back to shifted
        customizer.pipeline.folder_color_target.set_enabled(true);
        let re_enabled = customizer.render(16).unwrap();
        assert_eq!(
            re_enabled.data.get_pixel(0, 0).0,
            ct_pixel,
            "Re-enabling color target should restore shifted result"
        );
    }
}
