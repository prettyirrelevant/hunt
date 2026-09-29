pub mod ats;
pub mod feeds;
pub mod freehire;

use futures::{StreamExt, stream};
use reqwest::{Client, RequestBuilder};

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
            Ok(postings) => harvest.postings.extend(postings.into_iter().filter_map(Posting::with_web_links)),
            Err(err) => harvest.failures.push((source.name(), format!("{err:#}"))),
        }
    }
    harvest
}

const MAX_BODY: usize = 64 * 1024 * 1024;

async fn body(request: RequestBuilder) -> anyhow::Result<Vec<u8>> {
    let mut response = request.send().await?.error_for_status()?;
    let url = response.url().clone();
    let fits = |size: usize| -> anyhow::Result<usize> {
        anyhow::ensure!(size <= MAX_BODY, "{url} sent more than {} MB", MAX_BODY >> 20);
        Ok(size)
    };
    let mut body = Vec::with_capacity(fits(response.content_length().unwrap_or(0) as usize)?);
    while let Some(chunk) = response.chunk().await? {
        fits(body.len() + chunk.len())?;
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn fetch<'a>(source: &'a Source, http: &Client, want: &Want) -> (&'a Source, anyhow::Result<Vec<Posting>>) {
    (source, source.fetch(http, want).await)
}
