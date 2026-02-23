//! Icon types for cross-platform icon representation.
//!
//! This module provides types for representing system icons as a collection
//! of images at various sizes and scales.

use std::io::Cursor;

use image::{ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};

/// A rectangle defined in pixel coordinates.
///
/// Used to specify regions within an image, such as content bounds
/// that indicate where the actual icon content exists (excluding padding/margins).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RectPx {
    /// X offset from the left edge of the image
    pub x: u32,
    /// Y offset from the top edge of the image
    pub y: u32,
    /// Width of the rectangle
    pub width: u32,
    /// Height of the rectangle
    pub height: u32,
}

impl RectPx {
    /// Creates a new rectangle with the given position and dimensions.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Creates a rectangle starting at origin (0, 0) with the given dimensions.
    pub fn from_size(width: u32, height: u32) -> Self {
        Self { x: 0, y: 0, width, height }
    }

    /// Returns the right edge coordinate (x + width).
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    /// Returns the bottom edge coordinate (y + height).
    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }
}

/// A 2D size in pixel units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SizePx {
    pub width: u32,
    pub height: u32,
}

impl SizePx {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns true if width equals height.
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
}

/// A single icon image with its associated metadata.
///
/// Icon sets typically contain multiple images at different sizes and scales.
/// For example, macOS uses @1x and @2x variants, Windows uses multiple sizes
/// (16x16, 32x32, 48x48, 256x256), and Linux icon themes have similar patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct IconImage {
    /// The image data in RGBA format.
    pub data: RgbaImage,

    /// The display scale factor.
    ///
    /// - 1.0 for standard resolution (@1x)
    /// - 2.0 for retina/HiDPI (@2x)
    /// - 3.0 for @3x, etc.
    ///
    /// The "logical" size of the icon is `dimensions / scale`.
    pub scale: f32,

    /// The region within the image that contains the actual icon content.
    ///
    /// This is useful for icons that have built-in padding or margins.
    /// If the icon fills the entire image, this will equal
    /// `RectPx::from_size(width, height)`.
    pub content_bounds: RectPx,
}

impl IconImage {
    /// Creates a new icon image with the given data and metadata.
    pub fn new(data: RgbaImage, scale: f32, content_bounds: RectPx) -> Self {
        Self {
            data,
            scale,
            content_bounds,
        }
    }

    /// Creates a new icon image assuming content fills the entire image.
    pub fn new_full_content(data: RgbaImage, scale: f32) -> Self {
        let content_bounds = RectPx::from_size(data.width(), data.height());
        Self::new(data, scale, content_bounds)
    }

    /// Returns the pixel dimensions of the image.
    pub fn dimensions(&self) -> SizePx {
        SizePx::new(self.data.width(), self.data.height())
    }

    /// Returns the logical size of the icon (dimensions / scale).
    ///
    /// For a 64x64 @2x icon, the logical size is 32x32.
    pub fn logical_size(&self) -> (f32, f32) {
        (
            self.data.width() as f32 / self.scale,
            self.data.height() as f32 / self.scale,
        )
    }
}

/// A collection of icon images representing a single icon at various sizes and scales.
///
/// System icons typically come as a set of images at different resolutions.
/// This struct groups them together as a cohesive unit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IconSet {
    /// The individual icon images, typically at various sizes/scales.
    pub images: Vec<IconImage>,
}

impl IconSet {
    /// Creates a new empty icon set.
    pub fn new() -> Self {
        Self { images: Vec::new() }
    }

    /// Creates an icon set from a vector of images.
    pub fn from_images(images: Vec<IconImage>) -> Self {
        Self { images }
    }

    /// Adds an image to the icon set.
    pub fn add_image(&mut self, image: IconImage) {
        self.images.push(image);
    }

    /// Returns the number of images in the set.
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// Returns true if the icon set contains no images.
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// Finds an image by its logical size (closest match).
    ///
    /// This is useful when you need a specific size for display
    /// and want to find the best available variant.
    pub fn find_by_logical_size(&self, target_size: u32) -> Option<&IconImage> {
        self.images.iter().min_by_key(|img| {
            let (logical_w, _) = img.logical_size();
            (logical_w - target_size as f32).abs() as u32
        })
    }

    /// Returns an iterator over the icon images.
    pub fn iter(&self) -> impl Iterator<Item = &IconImage> {
        self.images.iter()
    }
}

impl IntoIterator for IconSet {
    type Item = IconImage;
    type IntoIter = std::vec::IntoIter<IconImage>;

    fn into_iter(self) -> Self::IntoIter {
        self.images.into_iter()
    }
}

impl<'a> IntoIterator for &'a IconSet {
    type Item = &'a IconImage;
    type IntoIter = std::slice::Iter<'a, IconImage>;

    fn into_iter(self) -> Self::IntoIter {
        self.images.iter()
    }
}

/// The RGB color of an icon's primary content surface.
///
/// Used as the reference point when computing color target deltas.
/// Each platform defines its own surface color (e.g., RGB(255, 217, 112)
/// for the golden-yellow Windows folder icon).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SurfaceColor {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
}

impl SurfaceColor {
    /// Creates a new surface color from RGB values.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// A base icon set combined with metadata about the icon's appearance.
///
/// This is the primary input to [`FolderIconCustomizer`](crate::FolderIconCustomizer),
/// pairing the icon images with the surface color needed to compute
/// color target deltas from target colors.
#[derive(Debug, Clone, PartialEq)]
pub struct FolderIconBase {
    /// The base icon images at various sizes.
    pub icons: IconSet,
    /// The HSL color of the icon's primary content surface.
    pub surface_color: SurfaceColor,
}

impl FolderIconBase {
    /// Creates a new icon base with the given icons and surface color.
    pub fn new(icons: IconSet, surface_color: SurfaceColor) -> Self {
        Self {
            icons,
            surface_color,
        }
    }
}

// ============================================================================
// Serializable transfer types
// ============================================================================

/// PNG-encoded representation of an [`IconImage`] for serialization and IPC.
///
/// Unlike [`IconImage`] (which holds `RgbaImage` in memory), this type stores
/// the image as PNG bytes, making it suitable for JSON serialization,
/// Tauri IPC, and wasm-bindgen transfer.
///
/// Use `TryFrom<&IconImage>` to convert, or [`SerializableFolderIconBase::try_from`]
/// to convert an entire [`FolderIconBase`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SerializableIconImage {
    /// PNG-encoded image bytes.
    pub png_data: Vec<u8>,
    /// Display scale factor (1.0 for @1x, 2.0 for @2x, etc.)
    pub scale: f32,
    /// Pixel width of the image.
    pub width: u32,
    /// Pixel height of the image.
    pub height: u32,
}

/// Serializable representation of an [`FolderIconBase`] for IPC transfer.
///
/// Contains everything needed to reconstruct an [`FolderIconBase`] on the
/// receiving end (e.g., in a WASM `CanvasRenderer`).
///
/// Use `TryFrom<&FolderIconBase>` to convert from an in-memory [`FolderIconBase`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SerializableFolderIconBase {
    /// PNG-encoded icon images at various sizes/scales.
    pub images: Vec<SerializableIconImage>,
    /// The surface color of the base icon.
    pub surface_color: SurfaceColor,
}

impl TryFrom<&IconImage> for SerializableIconImage {
    type Error = image::ImageError;

    fn try_from(img: &IconImage) -> std::result::Result<Self, Self::Error> {
        let mut png_bytes = Cursor::new(Vec::new());
        img.data.write_to(&mut png_bytes, ImageFormat::Png)?;

        Ok(Self {
            png_data: png_bytes.into_inner(),
            scale: img.scale,
            width: img.data.width(),
            height: img.data.height(),
        })
    }
}

impl TryFrom<&FolderIconBase> for SerializableFolderIconBase {
    type Error = image::ImageError;

    fn try_from(base: &FolderIconBase) -> std::result::Result<Self, Self::Error> {
        let images = base
            .icons
            .iter()
            .map(SerializableIconImage::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Self {
            images,
            surface_color: base.surface_color,
        })
    }
}

impl SerializableFolderIconBase {
    /// Decodes the PNG images and reconstructs an [`FolderIconBase`].
    ///
    /// This is the inverse of `TryFrom<&FolderIconBase>`.
    pub fn into_folder_icon_base(self) -> Result<FolderIconBase, image::ImageError> {
        let mut icon_set = IconSet::new();

        for img in &self.images {
            let rgba = image::load_from_memory(&img.png_data)?.to_rgba8();
            let width = rgba.width();
            let height = rgba.height();
            icon_set.add_image(IconImage::new(rgba, img.scale, RectPx::from_size(width, height)));
        }

        Ok(FolderIconBase::new(icon_set, self.surface_color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_px_new() {
        let rect = RectPx::new(10, 20, 100, 200);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 200);
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 220);
    }

    #[test]
    fn size_px_is_square() {
        assert!(SizePx::new(100, 100).is_square());
        assert!(!SizePx::new(100, 200).is_square());
    }

    #[test]
    fn icon_image_logical_size() {
        let img = IconImage::new_full_content(
            RgbaImage::new(64, 64),
            2.0,
        );
        let (w, h) = img.logical_size();
        assert_eq!(w, 32.0);
        assert_eq!(h, 32.0);
    }

    #[test]
    fn icon_set_operations() {
        let mut set = IconSet::new();
        assert!(set.is_empty());

        set.add_image(IconImage::new_full_content(
            RgbaImage::new(16, 16),
            1.0,
        ));
        set.add_image(IconImage::new_full_content(
            RgbaImage::new(32, 32),
            1.0,
        ));

        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());

        // Find closest to 20x20 logical size
        let found = set.find_by_logical_size(20).unwrap();
        // Should find the 16x16 since |16-20| < |32-20|
        assert_eq!(found.dimensions().width, 16);
    }
}
