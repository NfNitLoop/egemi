use std::{sync::Arc, time::Duration};

use mime::Mime;
use tokio::task::JoinHandle;

use super::{Result, Error};

use crate::{browser::network::{rt, Body, LoadedResource, Status}, util::DisplayJoin as _};



/// Knows how to load http/https.
#[derive(Debug)]
pub struct HttpLoader {
    // TODO: Re-use a client to get HTTP/2 & 3 speedups.

    max_size: Option<usize>,

    // TODO: WHen we support multiple tabs, we could just make a global client? LazyLock.
    client: reqwest::Client,

    // Which content types to request. If we don't get one of these back, then error out fast.
    accept_content_types: Vec<Mime>,
}

impl Default for HttpLoader {
    fn default() -> Self {
        Self { 
            max_size: Some(1024*1024 * 100), // 100 MiB
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .user_agent(USER_AGENT)
                .build()
                .expect("Building reqwest client"),
            accept_content_types: [
                // See: https://developer.mozilla.org/en-US/docs/Glossary/Quality_values
                "text/gemini; q=1",
                "text/markdown; q=0.9",
                "text/plain; q=0.8",
                // TODO: text/html once we can scrape actual text out of it.
            ].into_iter().map(|it| it.parse().expect("parsing mime")).collect(),
        }
    }
}

const USER_AGENT: &str = concat!(
    "eGemi v", env!("CARGO_PKG_VERSION")
);

impl HttpLoader {
    pub fn fetch(self: &Arc<Self>, url: &str) -> JoinHandle<Result<LoadedResource>> {
        let url = url.to_string();
        let fut = self.clone()._fetch(url);
        let rt = rt();
        rt.spawn(fut)
    }

    async fn _fetch(self: Arc<Self>, url: String) -> Result<LoadedResource> {
        let response = self.client.get(&url)
            .header("Accept", self.accept_content_types.iter().join(","))
            .send()
            .await?;

        let ctype = match response.headers().get("content-type") {
            Some(header) => match header.to_str() {
                Ok(str) => Some(str.to_owned()),
                Err(err) => Err(err)?,
            },
            None => None,
        };
        let ctype = match ctype {
            None => None,
            Some(ctype) => {
                Some(ctype.parse::<Mime>()?)
            }
        };

        // TODO: Check too big.

        // TODO: binary.
        let length = response.content_length();
        let status = Status::HttpStatus { 
            code: response.status().as_u16()
        };
        
        // For statuses like 301 or 404, we want to return the status code, and don't care
        // so much about the content. But for OK responses, we expect the content to be a type we requested.
        // If it's not, don't bother fetching it, return early.
        if status.ok() {
            let Some(mime) = &ctype else {
                return Err(Error::MissingContentType);
            };
            let type_match = self.accept_content_types.iter().any(|mt| {
                mt.essence_str() == mime.essence_str()
            });
            if !type_match {
                Err(Error::UnrequestedContentType(mime.clone()))?;
            }
        }

        let text = response.text().await?;


        let resource = LoadedResource {
            body: Body::Text(text.into()), 
            content_type: ctype.map(Into::into),
            length,
            status,
            url: url.into(),
        };

        Ok(resource)
    }
}