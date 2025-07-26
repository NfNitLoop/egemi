use std::sync::Arc;

use mime::Mime;
use tokio::task::JoinHandle;
use germ::request::non_blocking::request as germ_request;

use crate::browser::network::{rt, Body};

use super::{LoadedResource, Result, Error};



#[derive(Default, Debug)]
pub struct GeminiLoader {

}

impl GeminiLoader {
    pub fn fetch(self: &Arc<Self>, url: url::Url) -> JoinHandle<Result<LoadedResource>> {
        rt().spawn(self.clone()._fetch(url))
    }

    async fn _fetch(self: Arc<Self>, url: url::Url) -> Result<LoadedResource> {
        let response = match germ_request(&url).await {
            Ok(ok) => ok,
            Err(err) => Err(Error::Unknown(format!("{err:#?}")))?
        };

        let status = super::Status::HttpStatus {
            code: if *response.status() == germ::request::Status::Success {
                200
            } else { 500 } // TODO: better mapping here.
        };

        let ctype: Mime = response.meta().parse()?;

        Ok(LoadedResource {
            status,
            body: Body::Text(response.content().unwrap_or_else(String::new).into()),
            content_type: Some(Arc::new(ctype)),
            length: Some(*response.size() as u64),
            url: url.to_string().into()
        })
    }

}