//! Hmm, maybe "network" isn't a great module name.
//! This loader loads files from disk. 
//! If the path is a directory, it will return a gemtext directory listing.

use std::{io::ErrorKind, os::unix::fs::MetadataExt as _, path::PathBuf, sync::Arc};

use tokio::task::JoinHandle;
use url::Url;

use crate::browser::network::{rt, text_gemini, Body, Error, LoadedResource, Result, Status};

#[derive(Debug, Default)]
pub struct FileLoader;


impl FileLoader {
    pub fn fetch(self: &Arc<Self>, url: Url) -> JoinHandle<Result<LoadedResource>> {
        let fut = self.clone()._fetch(url);
        let rt = rt();
        rt.spawn(fut)
    }

    async fn _fetch(self: Arc<Self>, url: Url) -> Result<LoadedResource> {
        if url.scheme() != "file" {
            return Err(Error::InvalidUrl(String::from(url).into()));
        }
        let Ok(path) = url.to_file_path() else {
            return Err(Error::InvalidUrl(String::from(url).into()));
        };

        let stat = tokio::fs::metadata(&path).await;
        use ErrorKind::*;
        let stat = match stat {
            Err(err) if err.kind() == NotFound => return Ok(not_found(url)),
            Err(err) => Err(err)?,
            Ok(stat) => stat,
        };

        if stat.is_dir() {
            if !url.path().ends_with("/") {
                return dir_needs_slash(url);
            }
            return gemtext_dir_list(url, path).await;
        }

        let mebibyte: u64 = 1024 * 1024;

        if stat.is_file() {
            let bytes = stat.size();
            if bytes > 30 * mebibyte {
                return Err(Error::Unknown(format!("File too large: {bytes} bytes")));
            }
            return load_file(url, path).await;
        }

        // Symlinks not supported.

        Ok(not_found(url))
    }
}

async fn load_file(url: Url, path: PathBuf) -> std::result::Result<LoadedResource, Error> {
    let content_type = mime_guess::from_path(&path).first();
    let Some(content_type) = content_type else {
        return Err(Error::MissingContentType);
    };

    if content_type.type_() != "text" {
        return Err(Error::UnsupportedContentType(content_type))
    };

    let text = tokio::fs::read_to_string(path).await?;

    Ok(LoadedResource {
        body: Body::Text(text.into()),
        content_type: Some(content_type.into()),
        length: None,
        status: FileStatus::Ok.into(),
        url: String::from(url).into(),
    })
}

#[derive(Debug, PartialEq)]
pub enum FileStatus {
    // File found, all good:
    Ok,

    /// A directory was found, but the URL does not have a trailing slash.
    DirNeedsSlash,

    NotFound,

    TooBig { bytes: u64 },
}

impl Into<Status> for FileStatus {
    fn into(self) -> Status {
        Status::FileStatus(self)
    }
}

async fn gemtext_dir_list(url: Url, path: PathBuf) -> Result<LoadedResource> {
    let mut readdir = tokio::fs::read_dir(&path).await?;

    let mut dirs = Vec::<String>::new();
    let mut files = Vec::<String>::new();

    while let Some(entry) = readdir.next_entry().await? {
        let Ok(file_name) = entry.file_name().into_string() else { continue };
        let meta = entry.metadata().await?;
        if meta.is_dir() {
            dirs.push(file_name);
        } else if meta.is_file() {
            files.push(file_name);
        }
    }

    dirs.sort();
    files.sort();

    let mut out = String::new();

    if path.parent().is_some() {
        out.push_str("=> ../\n");
    }
    for dir in &dirs {
        out.push_str("=> ");
        out.push_str(&dir.replace(" ", "%20"));
        out.push_str("/\n")
    }
    if !dirs.is_empty() {
        out.push_str("\n");
    }

    for file in files {
        out.push_str("=> ");
        out.push_str(&file.replace(" ", "%20"));
        out.push_str("\n")
    }

    let loaded = LoadedResource {
        body: Body::Text(out.into()),
        content_type: Some(text_gemini()),
        length: None,
        status: FileStatus::Ok.into(),
        url: String::from(url).into(),
    };


    Ok(loaded)
}

fn not_found(url: Url) -> LoadedResource {
    LoadedResource{
        body: Body::Text("No such file".into()),
        content_type: Some(mime::TEXT_PLAIN.into()),
        length: None,
        status: FileStatus::NotFound.into(),
        url: String::from(url).into()
    }
}

fn dir_needs_slash(url: Url) -> Result<LoadedResource> {
    let segs = url.path_segments().expect("path segments");
    let last_seg = segs.rev().next().expect("at least one path segment");

    let mut out = String::new();
    out.push_str("# File Not Found\n\n");
    out.push_str("... but we found a directory. Did you mean:\n\n");
    out.push_str("=> ");
    out.push_str(&last_seg.replace(" ", "%20"));
    out.push_str("/");

    Ok(LoadedResource{
        body: Body::Text(out.into()),
        content_type: Some(text_gemini()),
        length: None,
        status: FileStatus::DirNeedsSlash.into(),
        url: String::from(url).into()
    })
}

