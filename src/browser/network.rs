//! Handlers for fetching resources from the network.

use std::{borrow::Cow, fmt::Display, sync::{Arc, LazyLock}, time::Duration};

use mime::Mime;
use reqwest::header::ToStrError;
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::util::DisplayJoin as _;

// A global runtime to execute async tasks on.
// The big benefit of async here is that tokio Tasks can be aborted at any time.
// Otherwise, an egui app is synchronous.

pub fn rt() -> Arc<Runtime> {
    static RT: LazyLock<Arc<Runtime>> = LazyLock::new(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()
            .expect("Tokio multithread init");
        Arc::new(rt)
    });

    RT.clone()
}

/// Knows how to load http/https.
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
    pub fn fetch(self: Arc<Self>, url: &str) -> JoinHandle<Result<LoadedResource>> {
        let url = url.to_string();
        let fut = self._fetch(url);
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
                Err(Error::UnsupportedContentType(mime.clone()))?;
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

// TODO: Worth using a strings/bytes crate for these?
pub type SCow = Cow<'static, str>;
pub type BCow = Cow<'static, [u8]>;

/// Resource that has been completely loaded and is ready for synchronous use.
#[derive(Debug)]
pub  struct LoadedResource {
    pub url: SCow,

    pub status: Status,

    // TODO: headers
    pub length: Option<u64>,
    pub content_type: Option<Arc<Mime>>,

    // TODO: 
    pub body: Body

    // TODO: Cert info.
}


/// Like an HTTP status, but might apply to not-HTTP.
#[derive(Debug)]
pub enum Status {
    HttpStatus {
        code: u16,
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::HttpStatus { code } => {
                write!(f, "HTTP {code}")
            },
        }
    }
}

impl Status {
    pub fn ok(&self) -> bool {
        use Status::*;
        match self {
            HttpStatus { code } => { 200 <= *code && *code < 300 }
        }
    }
}

#[derive(Debug)]
pub enum Body {
    Bytes(BCow),
    Text(SCow)
}



#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unknown error {0}")]
    Unknown(String),

    #[error("Unsupported Content-Type: {0}")]
    UnsupportedContentType(Mime),

    #[error("Missing Content-Type")]
    MissingContentType,

    #[error("Error parsing mime type {0}")]
    MimeParseError(#[from] mime::FromStrError),
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::Unknown(format!("{:?}", value))
    }
}

impl From <ToStrError> for Error {
    fn from(value: ToStrError) -> Self {
        Error::Unknown(format!("{:?}", value))
    }
}

pub type Result<T = ()> = std::result::Result<T, Error>;

