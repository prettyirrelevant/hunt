pub mod ats;
pub mod feeds;
pub mod freehire;

use futures::{StreamExt, stream};
use reqwest::Client;

use super::model::{Harvest, Want};
use crate::jobs::Posting;
use ats::Board;
use feeds::Feed;

pub enum Source {
    Freehire,
    Feed(Feed),
    Company(Board),
}

impl Source {
    pub fn all(watched: Vec<Board>) -> Vec<Source> {
        let feeds = Feed::ALL.into_iter().map(Source::Feed);
        let companies = watched.into_iter().map(Source::Company);
        std::iter::once(Source::Freehire).chain(feeds).chain(companies).collect()
    }

    pub fn name(&self) -> String {
        match self {
            Source::Freehire => "freehire".into(),
            Source::Feed(feed) => feed.name().into(),
            Source::Company(board) => board.to_string(),
        }
    }

    async fn fetch(&self, http: &Client, want: &Want) -> anyhow::Result<Vec<Posting>> {
        match self {
            Source::Freehire => freehire::search(http, want).await,
            Source::Feed(feed) => feed.fetch(http, want).await,
            Source::Company(board) => board.fetch(http).await,
        }
    }
}

pub async fn harvest(http: &Client, want: &Want, sources: &[Source]) -> Harvest {
    let mut harvest = Harvest { sources: sources.len(), ..Default::default() };
    let requests: Vec<_> = sources.iter().map(|source| fetch(source, http, want)).collect();
    let mut results = stream::iter(requests).buffer_unordered(6);
    while let Some((source, result)) = results.next().await {
        match result {
            Ok(postings) => harvest.postings.extend(postings),
            Err(err) => harvest.failures.push((source.name(), format!("{err:#}"))),
        }
    }
    harvest
}

async fn fetch<'a>(source: &'a Source, http: &Client, want: &Want) -> (&'a Source, anyhow::Result<Vec<Posting>>) {
    (source, source.fetch(http, want).await)
}
