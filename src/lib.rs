//! folco-renderer: Cross-platform icon customization library
//!
//! This crate provides utilities for loading system icons and applying
//! customizations such as hue rotations and SVG overlays.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{IconCustomizer, IconSet, HueRotationConfig, DecalConfig};
//!
//! let base_icons = IconSet::new();
//! let mut customizer = IconCustomizer::new(base_icons);
//!
//! // Configure layers directly through the pipeline
//! customizer.pipeline.hue.set_config(Some(HueRotationConfig::new(180.0)));
//! customizer.pipeline.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
//!
//! // Toggle layers without losing config
//! customizer.pipeline.hue.set_enabled(false);
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
//!     IconCustomizer, IconSet, Configurable,
//!     CustomizationProfile, HueRotationSettings,
//! };
//!
//! let mut customizer = IconCustomizer::new(IconSet::new());
//!
//! // Apply a profile
//! let profile = CustomizationProfile::new()
//!     .with_hue_rotation(HueRotationSettings { degrees: 180.0, enabled: true });
//! customizer.apply_profile(&profile);
//!
//! // Export current settings
//! let exported = customizer.export_profile();
//! let json = exported.to_json().unwrap();
//! ```

mod customizer;
mod icon;
mod layer;
mod profile;

pub use customizer::{Configurable, IconCustomizer};
pub use icon::{IconImage, IconSet, RectPx, SizePx};
pub use layer::{
    CacheKey, DecalConfig, DominantColor, HueRotationConfig, Layer, LayerConfig, LayerPipeline,
    LayerVersions, OverlayPosition, RenderContext, SvgOverlayConfig, SvgSource,
};
pub use profile::{
    CustomizationProfile, DecalSettings, HueRotationSettings, OverlaySettings,
    SerializablePosition, SerializableSvgSource,
};

