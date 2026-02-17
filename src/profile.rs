//! Serializable customization profile for cross-process/WASM communication.
//!
//! A [`CustomizationProfile`] captures all layer settings in a format that can
//! be serialized to JSON and sent between frontend (WASM/Tauri) and backend.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{
//!     CustomizationProfile, HslMutationSettings, DecalSettings, SerializableSvgSource,
//! };
//!
//! // Build a profile
//! let profile = CustomizationProfile::new()
//!     .with_hsl_mutation(HslMutationSettings {
//!         target_hue: 180.0,
//!         target_saturation: 0.8,
//!         target_lightness: 0.5,
//!         enabled: true,
//!     })
//!     .with_decal(DecalSettings {
//!         source: SerializableSvgSource::from_svg("<svg>...</svg>"),
//!         scale: 0.5,
//!         enabled: true,
//!     });
//!
//! // Serialize to JSON for sending to backend
//! let json = profile.to_json().unwrap();
//!
//! // Deserialize in backend
//! let restored = CustomizationProfile::from_json(&json).unwrap();
//! ```

use serde::{Deserialize, Serialize};

use crate::layer::{OverlayPosition, SvgSource};

// ============================================================================
// Serializable SVG Source
// ============================================================================

/// Serializable representation of an SVG source.
///
/// This enum serializes to a flat structure with either `svgData` or `emoji` field:
///
/// ```json
/// { "svgData": "<svg>...</svg>" }
/// // or
/// { "emoji": "🦆" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SerializableSvgSource {
    /// Raw SVG markup (mutually exclusive with `emoji` and `emoji_name`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg_data: Option<String>,

    /// Emoji character to resolve via twemoji (mutually exclusive with `svg_data` and `emoji_name`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,

    /// Emoji name to resolve via twemoji (mutually exclusive with `svg_data` and `emoji`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

impl SerializableSvgSource {
    /// Creates a source from raw SVG markup.
    pub fn from_svg(svg: impl Into<String>) -> Self {
        Self {
            svg_data: Some(svg.into()),
            emoji: None,
            emoji_name: None,
        }
    }

    /// Creates a source from an emoji character.
    pub fn from_emoji(emoji: impl Into<String>) -> Self {
        Self {
            svg_data: None,
            emoji: Some(emoji.into()),
            emoji_name: None,
        }
    }

    /// Creates a source from an emoji name (e.g., "duck").
    pub fn from_emoji_name(name: impl Into<String>) -> Self {
        Self {
            svg_data: None,
            emoji: None,
            emoji_name: Some(name.into()),
        }
    }
}

impl From<&SvgSource> for SerializableSvgSource {
    fn from(source: &SvgSource) -> Self {
        match source {
            SvgSource::Raw(svg) => Self::from_svg(svg),
            SvgSource::Emoji(emoji) => Self::from_emoji(emoji),
            SvgSource::EmojiName(name) => Self::from_emoji_name(name),
        }
    }
}

impl From<SerializableSvgSource> for SvgSource {
    fn from(source: SerializableSvgSource) -> Self {
        if let Some(emoji) = source.emoji {
            SvgSource::Emoji(emoji)
        } else if let Some(name) = source.emoji_name {
            SvgSource::EmojiName(name)
        } else if let Some(svg) = source.svg_data {
            SvgSource::Raw(svg)
        } else {
            SvgSource::Raw(String::new())
        }
    }
}

// ============================================================================
// Layer Settings (Serializable)
// ============================================================================

/// Serializable settings for HSL mutation layer.
///
/// Settings are expressed as a **target color** in HSL space. The renderer
/// computes the necessary deltas from the base icon's surface color.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HslMutationSettings {
    /// Target hue in degrees (0–360).
    pub target_hue: f32,

    /// Target saturation as a fraction (0.0–1.0).
    #[serde(default)]
    pub target_saturation: f32,

    /// Target lightness as a fraction (0.0–1.0).
    #[serde(default)]
    pub target_lightness: f32,

    /// Whether this layer is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Serializable settings for decal imprint layer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DecalSettings {
    /// The SVG source.
    #[serde(flatten)]
    pub source: SerializableSvgSource,

    /// Scale factor relative to the icon's content bounds (0.0-1.0).
    pub scale: f32,

    /// Whether this layer is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Serializable settings for SVG overlay layer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySettings {
    /// The SVG source.
    #[serde(flatten)]
    pub source: SerializableSvgSource,

    /// Position within the icon's content bounds.
    pub position: SerializablePosition,

    /// Scale factor relative to the icon's content bounds (0.0-1.0).
    pub scale: f32,

    /// Whether this layer is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Serializable version of [`OverlayPosition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SerializablePosition {
    BottomLeft,
    #[default]
    BottomRight,
    TopLeft,
    TopRight,
    Center,
}

impl From<OverlayPosition> for SerializablePosition {
    fn from(pos: OverlayPosition) -> Self {
        match pos {
            OverlayPosition::BottomLeft => Self::BottomLeft,
            OverlayPosition::BottomRight => Self::BottomRight,
            OverlayPosition::TopLeft => Self::TopLeft,
            OverlayPosition::TopRight => Self::TopRight,
            OverlayPosition::Center => Self::Center,
        }
    }
}

impl From<SerializablePosition> for OverlayPosition {
    fn from(pos: SerializablePosition) -> Self {
        match pos {
            SerializablePosition::BottomLeft => Self::BottomLeft,
            SerializablePosition::BottomRight => Self::BottomRight,
            SerializablePosition::TopLeft => Self::TopLeft,
            SerializablePosition::TopRight => Self::TopRight,
            SerializablePosition::Center => Self::Center,
        }
    }
}

fn default_true() -> bool {
    true
}

// ============================================================================
// CustomizationProfile
// ============================================================================

/// A serializable profile containing all customization settings.
///
/// This is the primary type for communicating settings between WASM frontend
/// and native backend. It captures layer configurations and enabled states
/// in a JSON-friendly format.
///
/// # JSON Format
///
/// ```json
/// {
///   "hslMutation": {
///     "targetHue": 180.0,
///     "targetSaturation": 0.8,
///     "targetLightness": 0.5,
///     "enabled": true
///   },
///   "decal": {
///     "svgData": "<svg>...</svg>",
///     "scale": 0.5,
///     "enabled": true
///   },
///   "overlay": null
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomizationProfile {
    /// HSL mutation layer settings. `None` means no config set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hsl_mutation: Option<HslMutationSettings>,

    /// Decal imprint layer settings. `None` means no config set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decal: Option<DecalSettings>,

    /// SVG overlay layer settings. `None` means no config set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay: Option<OverlaySettings>,
}

impl CustomizationProfile {
    /// Creates an empty profile with no layers configured.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets HSL mutation settings.
    pub fn with_hsl_mutation(mut self, settings: HslMutationSettings) -> Self {
        self.hsl_mutation = Some(settings);
        self
    }

    /// Sets decal settings.
    pub fn with_decal(mut self, settings: DecalSettings) -> Self {
        self.decal = Some(settings);
        self
    }

    /// Sets overlay settings.
    pub fn with_overlay(mut self, settings: OverlaySettings) -> Self {
        self.overlay = Some(settings);
        self
    }

    /// Serializes the profile to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serializes the profile to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes a profile from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_serialization_roundtrip() {
        let profile = CustomizationProfile::new()
            .with_hsl_mutation(HslMutationSettings {
                target_hue: 180.0,
                target_saturation: 0.8,
                target_lightness: 0.5,
                enabled: true,
            })
            .with_decal(DecalSettings {
                source: SerializableSvgSource::from_svg("<svg></svg>"),
                scale: 0.5,
                enabled: false,
            });

        let json = profile.to_json().unwrap();
        let restored = CustomizationProfile::from_json(&json).unwrap();

        assert_eq!(restored.hsl_mutation.as_ref().unwrap().target_hue, 180.0);
        assert_eq!(restored.hsl_mutation.as_ref().unwrap().target_saturation, 0.8);
        assert_eq!(restored.hsl_mutation.as_ref().unwrap().target_lightness, 0.5);
        assert!(restored.hsl_mutation.as_ref().unwrap().enabled);
        assert_eq!(
            restored.decal.as_ref().unwrap().source.svg_data.as_deref(),
            Some("<svg></svg>")
        );
        assert!(!restored.decal.as_ref().unwrap().enabled);
        assert!(restored.overlay.is_none());
    }

    #[test]
    fn profile_json_format() {
        let profile = CustomizationProfile::new().with_hsl_mutation(HslMutationSettings {
            target_hue: 90.0,
            target_saturation: 0.8,
            target_lightness: 0.5,
            enabled: true,
        });

        let json = profile.to_json_pretty().unwrap();

        // Verify camelCase serialization
        assert!(json.contains("\"hslMutation\""));
        assert!(json.contains("\"targetHue\""));
        assert!(json.contains("\"enabled\""));
    }

    #[test]
    fn profile_apply_to_customizer() {
        use crate::customizer::Configurable;
        use crate::icon::{IconBase, IconSet, SurfaceColor};
        use crate::IconCustomizer;

        let profile = CustomizationProfile::new()
            .with_hsl_mutation(HslMutationSettings {
                target_hue: 120.0,
                target_saturation: 0.8,
                target_lightness: 0.5,
                enabled: true,
            })
            .with_decal(DecalSettings {
                source: SerializableSvgSource::from_svg("test-svg"),
                scale: 0.3,
                enabled: false, // Disabled but config present
            });

        let mut customizer = IconCustomizer::new(IconBase::new(IconSet::new(), SurfaceColor::new(44.0, 1.0, 0.72)));
        customizer.apply_profile(&profile);

        // Check HSL — target values are stored directly
        assert!(customizer.pipeline.hsl.is_active());
        assert!((customizer.pipeline.hsl.config().unwrap().target_hue - 120.0).abs() < 0.01);

        // Check decal (has config but disabled)
        assert!(customizer.pipeline.decal.has_config());
        assert!(!customizer.pipeline.decal.is_enabled());
        assert!(!customizer.pipeline.decal.is_active());
        assert_eq!(
            customizer.pipeline.decal.config().unwrap().source,
            crate::layer::SvgSource::Raw("test-svg".into())
        );
    }

    #[test]
    fn profile_export_from_customizer() {
        use crate::customizer::Configurable;
        use crate::icon::{IconBase, IconSet, SurfaceColor};
        use crate::layer::HslMutationConfig;
        use crate::IconCustomizer;

        let surface = SurfaceColor::new(44.0, 1.0, 0.72);
        let mut customizer = IconCustomizer::new(IconBase::new(IconSet::new(), surface));
        customizer
            .pipeline
            .hsl
            .set_config(Some(HslMutationConfig::new(&surface, 89.0, 1.0, 0.648)));
        customizer.pipeline.hsl.set_enabled(false);

        let profile = customizer.export_profile();

        // Export reads target HSL directly from the config
        assert!(profile.hsl_mutation.is_some());
        assert!((profile.hsl_mutation.as_ref().unwrap().target_hue - 89.0).abs() < 0.01);
        assert!((profile.hsl_mutation.as_ref().unwrap().target_saturation - 1.0).abs() < 0.01);
        assert!((profile.hsl_mutation.as_ref().unwrap().target_lightness - 0.648).abs() < 0.01);
        assert!(!profile.hsl_mutation.as_ref().unwrap().enabled);
        assert!(profile.decal.is_none());
        assert!(profile.overlay.is_none());
    }

    #[test]
    fn overlay_position_serialization() {
        let profile = CustomizationProfile::new().with_overlay(OverlaySettings {
            source: SerializableSvgSource::from_svg("icon"),
            position: SerializablePosition::TopLeft,
            scale: 0.25,
            enabled: true,
        });

        let json = profile.to_json().unwrap();
        assert!(json.contains("\"top-left\""));

        let restored = CustomizationProfile::from_json(&json).unwrap();
        assert_eq!(
            restored.overlay.unwrap().position,
            SerializablePosition::TopLeft
        );
    }

    #[test]
    fn empty_profile_deserializes() {
        let json = "{}";
        let profile = CustomizationProfile::from_json(json).unwrap();

        assert!(profile.hsl_mutation.is_none());
        assert!(profile.decal.is_none());
        assert!(profile.overlay.is_none());
    }
}
