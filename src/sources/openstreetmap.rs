use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use super::{CrawlContext, CrawlResult, DataSource, SourceSchema};

/// OpenStreetMap source — crawls geographic features and tags via Overpass API.
/// TODO: implement proper Overpass QL crawling.
pub struct OpenStreetMapSource;

#[async_trait]
impl DataSource for OpenStreetMapSource {
    fn schema(&self) -> SourceSchema {
        SourceSchema {
            name: "openstreetmap".into(),
            description: "OpenStreetMap — editable world map with geographic features and tags"
                .into(),
            version: "0.1.0".into(),
            endpoint: "https://overpass-api.de/api/interpreter".into(),
            entity_types: vec![
                "node".into(),
                "way".into(),
                "relation".into(),
                "amenity".into(),
                "natural".into(),
            ],
            properties: vec![
                "connected_to".into(),
                "member_of".into(),
                "adjacent_to".into(),
            ],
            metadata_fields: HashMap::from([
                ("lat".into(), "Latitude".into()),
                ("lon".into(), "Longitude".into()),
                ("tags".into(), "OSM key-value tags".into()),
            ]),
            attribution: "ODbL — OpenStreetMap contributors".into(),
        }
    }

    async fn crawl(&self, _ctx: &CrawlContext) -> Result<CrawlResult> {
        anyhow::bail!("OpenStreetMap source not yet implemented. Coming soon.")
    }
}
