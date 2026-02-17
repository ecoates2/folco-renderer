//! folco-renderer: Cross-platform icon customization library
//!
//! This crate provides utilities for loading system icons and applying
//! customizations such as HSL mutations and SVG overlays.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{IconCustomizer, IconBase, IconSet, HslMutationConfig, DecalConfig, SurfaceColor};
//!
//! let surface = SurfaceColor::new(44.0, 1.0, 0.72);
//! let base = IconBase::new(IconSet::new(), surface);
//! let mut customizer = IconCustomizer::new(base);
//!
//! // Configure layers directly through the pipeline
//! customizer.pipeline.hsl.set_config(Some(HslMutationConfig::new(&surface, 200.0, 0.8, 0.5)));
//! customizer.pipeline.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
//!
//! // Toggle layers without losing config
//! customizer.pipeline.hsl.set_enabled(false);
//!
//! let output = customizer.render_all();
//! ```
//!
//! # Serializable Profiles
//!
//! For frontend-backend communication, use [`CustomizationProfile`]
//! with the [`Configurable`] trait. For WASM bindings, see the
//! `folco-renderer-wasm` crate.
//!
//! ```
//! use folco_renderer::{
//!     IconCustomizer, IconBase, IconSet, SurfaceColor, Configurable,
//!     CustomizationProfile, HslMutationSettings,
//! };
//!
//! let surface = SurfaceColor::new(44.0, 1.0, 0.72);
//! let mut customizer = IconCustomizer::new(IconBase::new(IconSet::new(), surface));
//!
//! // Apply a profile
//! let profile = CustomizationProfile::new()
//!     .with_hsl_mutation(HslMutationSettings {
//!         target_hue: 180.0, target_saturation: 0.8, target_lightness: 0.5, enabled: true,
//!     });
//! customizer.apply_profile(&profile);
//!
//! // Export current settings
//! let exported = customizer.export_profile();
//! let json = exported.to_json().unwrap();
//! ```

mod customizer;
mod error;
mod icon;
mod layer;
mod profile;

pub use customizer::{Configurable, IconCustomizer};
pub use error::RenderError;
pub use icon::{IconBase, IconImage, IconSet, RectPx, SizePx, SurfaceColor};
pub use layer::{
    CacheKey, DecalConfig, DominantColor, HslMutationConfig, Layer, LayerConfig, LayerPipeline,
    LayerVersions, OverlayPosition, RenderContext, SvgOverlayConfig, SvgSource,
};
pub use profile::{
    CustomizationProfile, DecalSettings, HslMutationSettings, OverlaySettings,
    SerializablePosition, SerializableSvgSource,
};

