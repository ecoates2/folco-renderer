//! Serializable customization profile for cross-process communication.
//!
//! A [`CustomizationProfile`] captures all layer settings in a format that can
//! be serialized to JSON and sent between frontend and backend processes.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{
//!     CustomizationProfile, HueRotationSettings, DecalSettings, SerializableSvgSource,
//! };
//!
//! // Build a profile
//! let profile = CustomizationProfile::new()
//!     .with_hue_rotation(HueRotationSettings { degrees: 180.0, enabled: true })
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
    /// Raw SVG markup (mutually exclusive with `emoji`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg_data: Option<String>,

    /// Emoji character to resolve via twemoji (mutually exclusive with `svg_data`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
}

impl SerializableSvgSource {
    /// Creates a source from raw SVG markup.
    pub fn from_svg(svg: impl Into<String>) -> Self {
        Self {
            svg_data: Some(svg.into()),
            emoji: None,
        }
    }

    /// Creates a source from an emoji character.
    pub fn from_emoji(emoji: impl Into<String>) -> Self {
        Self {
            svg_data: None,
            emoji: Some(emoji.into()),
        }
    }
}

impl From<&SvgSource> for SerializableSvgSource {
    fn from(source: &SvgSource) -> Self {
        match source {
            SvgSource::Raw(svg) => Self::from_svg(svg),
            SvgSource::Emoji(emoji) => Self::from_emoji(emoji),
        }
    }
}

impl From<SerializableSvgSource> for SvgSource {
    fn from(source: SerializableSvgSource) -> Self {
        if let Some(emoji) = source.emoji {
            SvgSource::Emoji(emoji)
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

/// Serializable settings for hue rotation layer.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HueRotationSettings {
    /// Rotation angle in degrees (0-360).
    pub degrees: f32,

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
/// This is the primary type for communicating settings between frontend
/// and backend processes. It captures layer configurations and enabled states
/// in a JSON-friendly format.
///
/// # JSON Format
///
/// ```json
/// {
///   "hueRotation": {
///     "degrees": 180.0,
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
    /// Hue rotation layer settings. `None` means no config set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue_rotation: Option<HueRotationSettings>,

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

    /// Sets hue rotation settings.
    pub fn with_hue_rotation(mut self, settings: HueRotationSettings) -> Self {
        self.hue_rotation = Some(settings);
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
            .with_hue_rotation(HueRotationSettings {
                degrees: 180.0,
                enabled: true,
            })
            .with_decal(DecalSettings {
                source: SerializableSvgSource::from_svg("<svg></svg>"),
                scale: 0.5,
                enabled: false,
            });

        let json = profile.to_json().unwrap();
        let restored = CustomizationProfile::from_json(&json).unwrap();

        assert_eq!(restored.hue_rotation.as_ref().unwrap().degrees, 180.0);
        assert!(restored.hue_rotation.as_ref().unwrap().enabled);
        assert_eq!(
            restored.decal.as_ref().unwrap().source.svg_data.as_deref(),
            Some("<svg></svg>")
        );
        assert!(!restored.decal.as_ref().unwrap().enabled);
        assert!(restored.overlay.is_none());
    }

    #[test]
    fn profile_json_format() {
        let profile = CustomizationProfile::new().with_hue_rotation(HueRotationSettings {
            degrees: 90.0,
            enabled: true,
        });

        let json = profile.to_json_pretty().unwrap();

        // Verify camelCase serialization
        assert!(json.contains("\"hueRotation\""));
        assert!(json.contains("\"degrees\""));
        assert!(json.contains("\"enabled\""));
    }

    #[test]
    fn profile_apply_to_customizer() {
        use crate::customizer::Configurable;
        use crate::icon::IconSet;
        use crate::IconCustomizer;

        let profile = CustomizationProfile::new()
            .with_hue_rotation(HueRotationSettings {
                degrees: 120.0,
                enabled: true,
            })
            .with_decal(DecalSettings {
                source: SerializableSvgSource::from_svg("test-svg"),
                scale: 0.3,
                enabled: false, // Disabled but config present
            });

        let mut customizer = IconCustomizer::new(IconSet::new());
        customizer.apply_profile(&profile);

        // Check hue
        assert!(customizer.pipeline.hue.is_active());
        assert_eq!(customizer.pipeline.hue.config().unwrap().degrees, 120.0);

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
        use crate::icon::IconSet;
        use crate::layer::HueRotationConfig;
        use crate::IconCustomizer;

        let mut customizer = IconCustomizer::new(IconSet::new());
        customizer
            .pipeline
            .hue
            .set_config(Some(HueRotationConfig::new(45.0)));
        customizer.pipeline.hue.set_enabled(false);

        let profile = customizer.export_profile();

        assert!(profile.hue_rotation.is_some());
        assert_eq!(profile.hue_rotation.as_ref().unwrap().degrees, 45.0);
        assert!(!profile.hue_rotation.as_ref().unwrap().enabled);
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

        assert!(profile.hue_rotation.is_none());
        assert!(profile.decal.is_none());
        assert!(profile.overlay.is_none());
    }
}
