//! Layer infrastructure for icon customization.
//!
//! This module provides the generic layer system used by `IconCustomizer`.
//! Each layer encapsulates a configuration, an enabled state, version tracking
//! for cache invalidation, and a per-size image cache.
//!
//! # Architecture
//!
//! Each layer config implements [`LayerEffect`], which defines:
//! - How the layer renders itself
//! - What properties it emits for downstream layers
//! - What properties it consumes from upstream layers
//!
//! Properties flow through the pipeline via [`RenderContext`], enabling
//! layers to communicate without tight coupling.

pub mod decal;
pub mod hue_rotation;
pub mod overlay;
pub mod svg;

pub use decal::DecalConfig;
pub use hue_rotation::HueRotationConfig;
pub use overlay::{OverlayPosition, SvgOverlayConfig};
pub use svg::SvgSource;

use crate::icon::IconImage;
use std::any::{Any, TypeId};
use std::collections::HashMap;

// ============================================================================
// Render Context
// ============================================================================

/// Context that flows through the rendering pipeline.
///
/// Layers can read properties set by upstream layers and emit new properties
/// for downstream layers to consume. This enables loose coupling between layers.
///
/// # Example
///
/// ```ignore
/// // Upstream layer emits a property
/// ctx.set(DominantColor(r, g, b, a));
///
/// // Downstream layer reads the property
/// if let Some(color) = ctx.get::<DominantColor>() {
///     // Use the color...
/// }
/// ```
pub struct RenderContext {
    /// The current image being processed through the pipeline.
    pub image: IconImage,

    /// Typed property bag for inter-layer communication.
    properties: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl RenderContext {
    /// Creates a new render context with the given base image.
    pub fn new(image: IconImage) -> Self {
        Self {
            image,
            properties: HashMap::new(),
        }
    }

    /// Sets a typed property that downstream layers can read.
    pub fn set<T: Any + Send + Sync>(&mut self, value: T) {
        self.properties.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Gets a typed property set by an upstream layer.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.properties
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }

    /// Checks if a property has been set.
    pub fn has<T: Any + Send + Sync>(&self) -> bool {
        self.properties.contains_key(&TypeId::of::<T>())
    }
}

// ============================================================================
// Common Properties
// ============================================================================

/// The dominant color sampled from the image.
///
/// Emitted by layers that modify the image appearance (like hue rotation).
/// Consumed by layers that need to derive colors from the image (like decal).
#[derive(Debug, Clone, Copy)]
pub struct DominantColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl DominantColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn as_tuple(&self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
}

// ============================================================================
// Layer Traits
// ============================================================================

/// Trait for layer configuration types.
///
/// Implementations must detect when a configuration meaningfully differs
/// from another, which drives cache invalidation.
pub trait LayerConfig: Clone {
    /// Returns true if this config differs from another in a way that
    /// would produce different rendering output.
    fn differs_from(&self, other: &Self) -> bool;
}

/// Trait for layer configurations that know how to apply themselves.
///
/// This is the core abstraction that makes layers self-contained. Each layer:
/// - Declares its upstream dependencies for cache invalidation
/// - Transforms the image in the render context
/// - Can read properties set by upstream layers
/// - Emits properties for downstream layers in a dedicated method
///
/// The separation of [`transform`](Self::transform) and [`emit`](Self::emit)
/// provides a canonical place for property emission and makes the data flow
/// explicit.
pub trait LayerEffect: LayerConfig {
    /// Returns the dependency version for cache invalidation.
    ///
    /// Layers that depend on upstream layers should combine their versions.
    /// Root layers (no dependencies) should return `DependencyVersion::NONE`.
    fn dependencies(versions: &LayerVersions) -> DependencyVersion;

    /// Transform the image in the render context.
    ///
    /// Implementations should:
    /// 1. Read any needed properties from `ctx` (set by upstream layers)
    /// 2. Modify `ctx.image` as needed
    ///
    /// Property emission happens in [`emit`](Self::emit), not here.
    fn transform(&self, ctx: &mut RenderContext);

    /// Emit properties for downstream layers to consume.
    ///
    /// Called after [`transform`](Self::transform). The default implementation
    /// emits nothing. Override this to emit properties like [`DominantColor`].
    ///
    /// Properties are emitted via `ctx.set()`.
    fn emit(&self, _ctx: &mut RenderContext) {}
}



// ============================================================================
// Layer Dependencies
// ============================================================================

/// Represents the combined version of upstream layer dependencies.
///
/// This is used to detect when a layer's cache is stale because an
/// upstream layer has changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DependencyVersion(u64);

impl DependencyVersion {
    /// No dependencies (root layer).
    pub const NONE: Self = Self(0);

    /// Creates a dependency version from a single version number.
    pub fn from_version(version: u64) -> Self {
        Self(version)
    }

    /// Creates a dependency version from a single upstream layer.
    pub fn from_layer<C: LayerConfig>(layer: &Layer<C>) -> Self {
        Self(layer.version())
    }

    /// Combines multiple upstream layer versions into one.
    pub fn combine(versions: &[u64]) -> Self {
        Self(versions.iter().fold(0u64, |acc, v| acc.wrapping_add(*v)))
    }
}

// ============================================================================
// Layer Versions
// ============================================================================

/// Snapshot of all layer versions in the pipeline.
///
/// Passed to [`LayerEffect::dependencies`] so each layer can declare
/// which upstream layers it depends on for cache invalidation.
#[derive(Debug, Clone, Copy)]
pub struct LayerVersions {
    /// Version of the hue rotation layer.
    pub hue: u64,
    /// Version of the decal layer.
    pub decal: u64,
    /// Version of the overlay layer.
    pub overlay: u64,
}

// ============================================================================
// CacheKey
// ============================================================================

/// Key for cached rendered images.
///
/// Uses width, height, and scale (as integer bits) to identify unique image sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    width: u32,
    height: u32,
    scale_bits: u32,
}

impl CacheKey {
    /// Creates a cache key for the given dimensions and scale.
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        Self {
            width,
            height,
            scale_bits: scale.to_bits(),
        }
    }

    /// Creates a cache key from an icon image.
    pub fn from_icon(icon: &IconImage) -> Self {
        Self::new(icon.data.width(), icon.data.height(), icon.scale)
    }
}

// ============================================================================
// Generic Layer
// ============================================================================

/// A generic layer with configuration, caching, and version tracking.
///
/// The layer tracks:
/// - Optional configuration of type `C`
/// - Whether the layer is enabled (can be toggled without losing config)
/// - A version number that increments on any state change
/// - A cache of rendered images keyed by size
/// - The dependency version when each cache entry was stored
pub struct Layer<C: LayerConfig> {
    config: Option<C>,
    enabled: bool,
    version: u64,
    cache: HashMap<CacheKey, (IconImage, u64)>,
}

impl<C: LayerConfig> Default for Layer<C> {
    fn default() -> Self {
        Self {
            config: None,
            enabled: true,
            version: 0,
            cache: HashMap::new(),
        }
    }
}

impl<C: LayerConfig> Layer<C> {
    /// Returns the current configuration, if any.
    pub fn config(&self) -> Option<&C> {
        self.config.as_ref()
    }

    /// Returns true if this layer is active (has config AND is enabled).
    pub fn is_active(&self) -> bool {
        self.enabled && self.config.is_some()
    }

    /// Returns true if the layer has a configuration set.
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }

    /// Returns whether the layer is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether the layer is enabled.
    ///
    /// Returns true if the enabled state changed.
    pub fn set_enabled(&mut self, enabled: bool) -> bool {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.version = self.version.wrapping_add(1);
            self.cache.clear();
            true
        } else {
            false
        }
    }

    /// Returns the current version number.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Sets the configuration. Returns true if it changed.
    ///
    /// Clears the cache and increments version if the config differs.
    pub fn set_config(&mut self, config: Option<C>) -> bool {
        let differs = match (&self.config, &config) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(old), Some(new)) => old.differs_from(new),
        };

        if differs {
            self.config = config;
            self.version = self.version.wrapping_add(1);
            self.cache.clear();
            true
        } else {
            false
        }
    }

    /// Invalidates the cache and increments version.
    ///
    /// Called when upstream layers change.
    pub fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.cache.clear();
    }

    /// Gets a cached image if valid for the given key and dependency version.
    pub fn get_cached(&self, key: CacheKey, deps: DependencyVersion) -> Option<&IconImage> {
        self.cache.get(&key).and_then(|(img, stored_dep)| {
            if *stored_dep == deps.0 {
                Some(img)
            } else {
                None
            }
        })
    }

    /// Stores an image in the cache with the current dependency version.
    pub fn store(&mut self, key: CacheKey, image: IconImage, deps: DependencyVersion) {
        self.cache.insert(key, (image, deps.0));
    }
}

impl<C: LayerEffect> Layer<C> {
    /// Apply this layer to the render context, using cache if valid.
    ///
    /// If the layer is not active, it does nothing (context passes through unchanged).
    /// If a valid cached result exists, it updates the context image from cache.
    /// Otherwise, it calls transform() then emit() and caches the result.
    pub fn apply(&mut self, ctx: &mut RenderContext, key: CacheKey, versions: &LayerVersions) {
        if !self.is_active() {
            return;
        }

        // Compute dependencies from the trait
        let deps = C::dependencies(versions);

        // Check cache first
        if let Some(cached) = self.get_cached(key, deps) {
            ctx.image = cached.clone();
            // Re-emit properties (they aren't cached, only the image is)
            self.config().unwrap().emit(ctx);
            return;
        }

        // Apply the layer: transform then emit
        let config = self.config().unwrap();
        config.transform(ctx);
        config.emit(ctx);

        // Cache the result (image only, properties are re-emitted on cache hit)
        self.store(key, ctx.image.clone(), deps);
    }
}

// ============================================================================
// Composite Layer
// ============================================================================

/// A cache-only layer for final composited images.
///
/// Unlike [`Layer<C>`], this has no configuration or enabled state.
/// It purely caches the final rendered output and tracks a version
/// for invalidation when any upstream layer changes.
pub struct CompositeLayer {
    version: u64,
    cache: HashMap<CacheKey, (IconImage, u64)>,
}

impl Default for CompositeLayer {
    fn default() -> Self {
        Self {
            version: 0,
            cache: HashMap::new(),
        }
    }
}

impl CompositeLayer {
    /// Returns the current version number.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Invalidates the cache and increments version.
    pub fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.cache.clear();
    }

    /// Gets a cached image if valid for the given key and dependency version.
    pub fn get_cached(&self, key: CacheKey, deps: DependencyVersion) -> Option<&IconImage> {
        self.cache.get(&key).and_then(|(img, stored_dep)| {
            if *stored_dep == deps.0 {
                Some(img)
            } else {
                None
            }
        })
    }

    /// Stores an image in the cache with the current dependency version.
    pub fn store(&mut self, key: CacheKey, image: IconImage, deps: DependencyVersion) {
        self.cache.insert(key, (image, deps.0));
    }
}

// ============================================================================
// Layer Pipeline
// ============================================================================

/// Defines the layer pipeline with explicit dependency relationships.
///
/// This struct encapsulates all layers and their dependencies, ensuring that
/// cache invalidation propagates correctly through the pipeline.
///
/// # Dependency Graph
///
/// ```text
/// Base Image
///     │
///     ▼
/// ┌─────────┐
/// │   Hue   │ ◄── No dependencies (root layer)
/// └────┬────┘
///      │
///      ▼
/// ┌─────────┐
/// │  Decal  │ ◄── Depends on: Hue
/// └────┬────┘
///      │
///      ▼
/// ┌─────────┐
/// │ Overlay │ ◄── No direct dependencies (applied last)
/// └────┬────┘
///      │
///      ▼
/// ┌─────────────┐
/// │  Composite  │ ◄── Depends on: Hue + Decal + Overlay
/// └─────────────┘
/// ```
pub struct LayerPipeline {
    /// Hue rotation layer (root - no dependencies).
    pub hue: Layer<HueRotationConfig>,

    /// Decal imprint layer (depends on hue).
    pub decal: Layer<DecalConfig>,

    /// SVG overlay layer (no dependencies, applied last).
    pub overlay: Layer<SvgOverlayConfig>,

    /// Composite cache (depends on all layers).
    pub composite: CompositeLayer,
}

impl Default for LayerPipeline {
    fn default() -> Self {
        Self {
            hue: Layer::default(),
            decal: Layer::default(),
            overlay: Layer::default(),
            composite: CompositeLayer::default(),
        }
    }
}

impl LayerPipeline {
    /// Returns a snapshot of all layer versions.
    ///
    /// Used by [`LayerEffect::dependencies`] to compute cache invalidation.
    pub fn layer_versions(&self) -> LayerVersions {
        LayerVersions {
            hue: self.hue.version(),
            decal: self.decal.version(),
            overlay: self.overlay.version(),
        }
    }

    /// Invalidates all caches.
    pub fn invalidate_all(&mut self) {
        self.hue.invalidate();
        self.decal.invalidate();
        self.overlay.invalidate();
        self.composite.invalidate();
    }

    /// Returns the combined dependency version for the composite layer.
    fn composite_dependencies(&self) -> DependencyVersion {
        DependencyVersion::combine(&[
            self.hue.version(),
            self.decal.version(),
            self.overlay.version(),
        ])
    }

    /// Renders an icon through the full layer pipeline.
    ///
    /// This is the main entry point for rendering. It:
    /// 1. Checks the composite cache first
    /// 2. Creates a render context with the base image
    /// 3. Applies each layer in order, passing the context through
    /// 4. Caches and returns the final result
    pub fn render(&mut self, base: &IconImage) -> IconImage {
        let key = CacheKey::from_icon(base);
        let composite_deps = self.composite_dependencies();

        // Check composite cache first
        if let Some(cached) = self.composite.get_cached(key, composite_deps) {
            return cached.clone();
        }

        // Create render context
        let mut ctx = RenderContext::new(base.clone());

        // Apply layers in order (each layer computes its own dependencies)
        let versions = self.layer_versions();
        self.hue.apply(&mut ctx, key, &versions);
        self.decal.apply(&mut ctx, key, &versions);
        self.overlay.apply(&mut ctx, key, &versions);

        // Cache the final result
        self.composite.store(key, ctx.image.clone(), composite_deps);

        ctx.image
    }
}
