use anyhow::{Result, ensure};

pub const DEFAULT_RESOURCE_DIMENSIONS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(u16);

impl ResourceId {
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyPoint(Box<[f32]>);

impl PropertyPoint {
    pub fn new(values: impl Into<Box<[f32]>>) -> Result<Self> {
        let values = values.into();
        ensure!(
            !values.is_empty(),
            "a property point needs at least one dimension"
        );
        ensure!(
            values.iter().all(|value| value.is_finite()),
            "property coordinates must be finite"
        );
        Ok(Self(values))
    }

    #[inline]
    pub fn coordinates(&self) -> &[f32] {
        &self.0
    }
}

#[derive(Debug)]
pub struct Resources {
    dimensions: usize,
    entries: Vec<PropertyPoint>,
}

impl Resources {
    pub fn new(dimensions: usize) -> Self {
        assert!(
            dimensions > 0,
            "resource space needs at least one dimension"
        );
        Self {
            dimensions,
            entries: Vec::new(),
        }
    }

    pub fn register(&mut self, properties: PropertyPoint) -> Result<ResourceId> {
        ensure!(
            properties.coordinates().len() == self.dimensions,
            "resource has {} dimensions, expected {}",
            properties.coordinates().len(),
            self.dimensions
        );
        let id = ResourceId(
            u16::try_from(self.entries.len())
                .map_err(|_| anyhow::anyhow!("primitive resource ID space is exhausted"))?,
        );
        self.entries.push(properties);
        Ok(id)
    }

    #[inline]
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    #[inline]
    pub fn get(&self, id: ResourceId) -> Option<&PropertyPoint> {
        self.entries.get(id.index())
    }

    pub fn distance(&self, left: ResourceId, right: ResourceId) -> Result<f32> {
        let left = self
            .get(left)
            .ok_or_else(|| anyhow::anyhow!("unknown resource {left:?}"))?;
        let right = self
            .get(right)
            .ok_or_else(|| anyhow::anyhow!("unknown resource {right:?}"))?;
        Ok(euclidean_distance(left.coordinates(), right.coordinates()))
    }

    pub fn distance_to_point(&self, resource: ResourceId, point: &PropertyPoint) -> Result<f32> {
        ensure!(
            point.coordinates().len() == self.dimensions,
            "ideal point has {} dimensions, expected {}",
            point.coordinates().len(),
            self.dimensions
        );
        let resource = self
            .get(resource)
            .ok_or_else(|| anyhow::anyhow!("unknown resource {resource:?}"))?;
        Ok(euclidean_distance(
            resource.coordinates(),
            point.coordinates(),
        ))
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new(DEFAULT_RESOURCE_DIMENSIONS)
    }
}

fn euclidean_distance(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f32>()
        .sqrt()
}
