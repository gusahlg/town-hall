//! The primitive-resource lattice.
//!
//! Conceptually every primitive resource is one point of an N-dimensional
//! property lattice; in memory a resource is only a flat [`ResourceId`]. The
//! eight ID bits are divided as evenly as possible between the dimensions, so
//! every possible `u8` encodes exactly one lattice point and coordinates are
//! derived on demand instead of stored.
//!
//! This module only answers how geometrically similar two primitive resources
//! are. Mapping distance onto desirability belongs to the future
//! need/configuration system, and resource-property space is unrelated to
//! world-map geography.

use anyhow::{Result, ensure};

/// Number of property dimensions in the resource lattice. Compile-time
/// configurable; must stay within `1..=8` so every dimension receives at
/// least one of the eight `ResourceId` bits.
pub const DIMENSIONS: usize = 8;

const RESOURCE_BITS: usize = u8::BITS as usize;
pub const RESOURCE_COUNT: usize = 1 << RESOURCE_BITS;

const _: () = assert!(
    1 <= DIMENSIONS && DIMENSIONS <= RESOURCE_BITS,
    "every lattice dimension needs at least one of the eight ResourceId bits"
);

/// A primitive resource: a flat `u8` compactly encoding one lattice point.
///
/// Numerical ResourceId distance DOES NOT represent resource similarity.
/// `ResourceId(63)` and `ResourceId(64)` are numerically adjacent yet can sit
/// far apart once the flat ID is folded into the lattice. Never compare
/// resources with `a.0.abs_diff(b.0)`; always use [`distance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(pub u8);

impl ResourceId {
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// How many of the eight ID bits `dimension` receives. Leading dimensions
/// absorb the remainder when `DIMENSIONS` does not divide eight evenly.
pub const fn bits_for_dimension(dimension: usize) -> usize {
    let base = RESOURCE_BITS / DIMENSIONS;
    let remainder = RESOURCE_BITS % DIMENSIONS;

    base + if dimension < remainder { 1 } else { 0 }
}

/// Number of discrete positions along `dimension`.
pub const fn resolution(dimension: usize) -> usize {
    1 << bits_for_dimension(dimension)
}

/// Derives the lattice coordinate implicitly encoded by `id`. The last
/// dimension is the least significant digit of the mixed-radix encoding.
pub fn coordinates(id: ResourceId) -> [u8; DIMENSIONS] {
    let mut coordinates = [0; DIMENSIONS];
    let mut value = id.0 as usize;

    let mut dimension = DIMENSIONS;
    while dimension > 0 {
        dimension -= 1;

        let radix = resolution(dimension);
        coordinates[dimension] = (value % radix) as u8;
        value /= radix;
    }

    coordinates
}

/// The resource at a lattice coordinate, or `None` when any coordinate falls
/// outside its dimension's resolution.
pub fn resource_at(coordinates: [u8; DIMENSIONS]) -> Option<ResourceId> {
    let mut value = 0usize;

    for (dimension, &coordinate) in coordinates.iter().enumerate() {
        let radix = resolution(dimension);

        if coordinate as usize >= radix {
            return None;
        }

        value *= radix;
        value += coordinate as usize;
    }

    Some(ResourceId(value as u8))
}

/// Squared geometric similarity distance across all lattice dimensions. Each
/// coordinate difference is normalized by that dimension's maximum possible
/// difference so unevenly sized dimensions weigh equally, and the sum is
/// divided by `DIMENSIONS` to keep the result within `[0, 1]`.
pub fn distance_squared(a: ResourceId, b: ResourceId) -> f32 {
    let a = coordinates(a);
    let b = coordinates(b);

    let mut distance = 0.0;

    for dimension in 0..DIMENSIONS {
        let max = (resolution(dimension) - 1) as f32;
        let difference = (a[dimension] as f32 - b[dimension] as f32) / max;

        distance += difference * difference;
    }

    distance / DIMENSIONS as f32
}

/// Normalized geometric distance in `[0, 1]`: zero for identical resources,
/// approaching one near opposite extremes of property space.
pub fn distance(a: ResourceId, b: ResourceId) -> f32 {
    distance_squared(a, b).sqrt()
}

/// An arbitrary ideal point in resource-property space. Unlike a
/// [`ResourceId`] it is not restricted to lattice points; the future need
/// system compares ideal points against actual resource coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyPoint([f32; DIMENSIONS]);

impl PropertyPoint {
    pub fn new(values: [f32; DIMENSIONS]) -> Result<Self> {
        ensure!(
            values.iter().all(|value| value.is_finite()),
            "property coordinates must be finite"
        );
        Ok(Self(values))
    }

    #[inline]
    pub fn coordinates(&self) -> &[f32; DIMENSIONS] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_resource() -> impl Iterator<Item = ResourceId> {
        (0..=u8::MAX).map(ResourceId)
    }

    #[test]
    fn dimension_resolutions_cover_the_whole_id_space() {
        let bits: usize = (0..DIMENSIONS).map(bits_for_dimension).sum();
        assert_eq!(bits, RESOURCE_BITS);
        let product: usize = (0..DIMENSIONS).map(resolution).product();
        assert_eq!(product, RESOURCE_COUNT);
    }

    #[test]
    fn every_resource_round_trips_through_its_coordinates() {
        for id in every_resource() {
            assert_eq!(resource_at(coordinates(id)), Some(id));
        }
    }

    #[test]
    fn out_of_bounds_coordinates_have_no_resource() {
        // With a single dimension every u8 coordinate is in bounds and
        // out-of-bounds coordinates are unrepresentable.
        if DIMENSIONS == 1 {
            return;
        }
        let mut coordinates = [0; DIMENSIONS];
        coordinates[DIMENSIONS - 1] = resolution(DIMENSIONS - 1) as u8;
        assert_eq!(resource_at(coordinates), None);
    }

    #[test]
    fn distance_from_a_resource_to_itself_is_zero() {
        for id in every_resource() {
            assert_eq!(distance(id, id), 0.0);
        }
    }

    #[test]
    fn distance_is_symmetric_and_normalized() {
        for a in every_resource() {
            for b in every_resource() {
                let forward = distance(a, b);
                assert_eq!(forward, distance(b, a));
                assert!((0.0..=1.0 + f32::EPSILON).contains(&forward));
            }
        }
    }

    #[test]
    fn opposite_corners_are_a_full_distance_apart() {
        let origin = resource_at([0; DIMENSIONS]).unwrap();
        let mut far = [0; DIMENSIONS];
        for (dimension, coordinate) in far.iter_mut().enumerate() {
            *coordinate = (resolution(dimension) - 1) as u8;
        }
        let far = resource_at(far).unwrap();
        assert!((distance(origin, far) - 1.0).abs() < 1e-6);
    }
}
