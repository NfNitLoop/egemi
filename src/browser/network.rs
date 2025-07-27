//! Handlers for fetching resources from the network.

pub mod http;
pub mod file;
pub mod gemini;

use std::{borrow::Cow, fmt::Display, io, sync::{Arc, LazyLock}, time::Duration};

use mime::Mime;
use reqwest::header::ToStrError;
use tokio::{runtime::Runtime, task::JoinHandle};
use url::Url;

use crate::{browser::network::{file::FileStatus, gemini::GeminiLoader, http::HttpLoader}, util::DisplayJoin as _};

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

#[derive(Default, Debug)]
pub struct MultiLoader {
    http: Arc<HttpLoader>,
    gemini: Arc<GeminiLoader>,
    file: Arc<file::FileLoader>,
}

impl MultiLoader {
    pub fn fetch(&self, url: SCow) -> JoinHandle<Result<LoadedResource>> {
        let parsed = match Url::parse(&url) {
            Ok(ok) => ok,
            Err(err) => {
                return async_err(Error::InvalidUrl(url))
            },
        };
        if parsed.scheme() == "gemini" {
            self.gemini.fetch(parsed)
        } else if parsed.scheme() == "http" || parsed.scheme() == "https" {
            self.http.fetch(&url)
        } else if parsed.scheme() == "file" {
            self.file.fetch(parsed)
        } else {
            async_err(Error::UnsupportedUrlScheme(parsed))
        }
    }
}

fn async_err(err: Error) -> JoinHandle<Result<LoadedResource>> {
    rt().spawn( async move {
        Err(err)
    })
}



// TODO: Worth using a strings/bytes crate for these?
pub type SCow = Cow<'static, str>;
pub type BCow = Cow<'static, [u8]>;

/// Resource that has been completely loaded and is ready for synchronous use.
#[derive(Debug)]
pub struct LoadedResource {
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
    },

    FileStatus(FileStatus),
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::HttpStatus { code } => {
                write!(f, "HTTP {code}")
            },
            Status::FileStatus(stat) => write!(f, "{stat:?}"),
        }
    }
}

impl Status {
    pub fn ok(&self) -> bool {
        use Status::*;
        match self {
            HttpStatus { code } => { 200 <= *code && *code < 300 },
            FileStatus(stat) => { stat == &file::FileStatus::Ok },
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

    #[error("Unsupported URL scheme: {0}")]
    UnsupportedUrlScheme(Url),

    /// The web server returned a content type we didn't request.
    #[error("Unrequested Content-Type: {0}")]
    UnrequestedContentType(Mime),

    #[error("Unsupported Content-Type: {0}")]
    UnsupportedContentType(Mime),


    #[error("Missing Content-Type")]
    MissingContentType,

    #[error("Invalid URL: {0}")]
    InvalidUrl(SCow),

    #[error("Error parsing mime type {0}")]
    MimeParseError(#[from] mime::FromStrError),

    #[error("I/O Error: {0}")]
    IoError(#[from] io::Error),
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

pub fn text_gemini() -> Arc<Mime> {
    use std::sync::LazyLock;

    static VALUE: LazyLock<Arc<Mime>> = LazyLock::new(|| {
        "text/gemini".parse::<Mime>().expect("text/gemini").into()
    });
    return VALUE.clone()
}