//! folco-renderer: Cross-platform icon customization library
//!
//! This crate provides utilities for loading system icons and applying
//! customizations such as color targeting and SVG overlays.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, FolderColorTargetConfig, DecalConfig, SurfaceColor};
//!
//! let surface = SurfaceColor::new(255, 217, 112);
//! let base = FolderIconBase::new(IconSet::new(), surface);
//! let mut customizer = FolderIconCustomizer::new(base);
//!
//! // Configure layers directly through the pipeline
//! customizer.pipeline.folder_color_target.set_config(Some(FolderColorTargetConfig::new(33, 150, 243)));
//! customizer.pipeline.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
//!
//! // Toggle layers without losing config
//! customizer.pipeline.folder_color_target.set_enabled(false);
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
//!     FolderIconCustomizer, FolderIconBase, IconSet, SurfaceColor, Configurable,
//!     CustomizationProfile, FolderColorTargetConfig,
//! };
//!
//! let surface = SurfaceColor::new(255, 217, 112);
//! let mut customizer = FolderIconCustomizer::new(FolderIconBase::new(IconSet::new(), surface));
//!
//! // Apply a profile
//! let profile = CustomizationProfile::new()
//!     .with_folder_color_target(FolderColorTargetConfig::new(33, 150, 243));
//! customizer.apply_profile(&profile);
//!
//! // Export current settings
//! let exported = customizer.export_profile();
//! let json = exported.to_json().unwrap();
//! ```

pub mod folder_color;
mod customizer;
mod error;
mod icon;
mod layer;
mod profile;

pub use customizer::{Configurable, FolderIconCustomizer};
pub use error::RenderError;
pub use icon::{
    FolderIconBase, IconImage, IconSet, RectPx, SerializableFolderIconBase, SerializableIconImage, SizePx,
    SurfaceColor,
};
pub use layer::{
    CacheKey, DecalConfig, DominantColor, FolderColorTargetConfig, Layer, LayerConfig,
    LayerPipeline, LayerVersions, OverlayPosition, RenderContext,
    SvgOverlayConfig, SvgSource,
};
pub use profile::CustomizationProfile;
pub use folder_color::{FolderColor, FolderColorMetadata};

