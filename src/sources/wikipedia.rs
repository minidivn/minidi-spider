use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use super::{CrawlContext, CrawlResult, DataSource, SourceSchema};

/// Wikipedia source — crawls article summaries and page links.
/// TODO: implement proper Wikipedia API crawling.
pub struct WikipediaSource;

#[async_trait]
impl DataSource for WikipediaSource {
    fn schema(&self) -> SourceSchema {
        SourceSchema {
            name: "wikipedia".into(),
            description: "Wikipedia — free encyclopedia articles with rich text summaries".into(),
            version: "0.1.0".into(),
            endpoint: "https://en.wikipedia.org/w/api.php".into(),
            entity_types: vec!["article".into(), "category".into()],
            properties: vec!["links_to".into(), "category_member".into()],
            metadata_fields: HashMap::from([
                ("page_id".into(), "Numeric Wikipedia page ID".into()),
                ("url".into(), "Full article URL".into()),
                ("summary".into(), "First paragraph of article".into()),
            ]),
            attribution: "CC-BY-SA — Wikipedia contributors".into(),
        }
    }

    async fn crawl(&self, ctx: &CrawlContext) -> Result<CrawlResult> {
        anyhow::bail!("Wikipedia source not yet implemented. Coming soon.")
    }
}
