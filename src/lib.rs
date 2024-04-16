use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
#[cfg(feature = "multipart")]
use reqwest::multipart::Form;
use reqwest::{Body, Client as VanillaClient, IntoUrl, Method, Request, Response};
use serde::Serialize;
use std::convert::TryFrom;
use std::fmt::Display;
use thiserror::Error;

#[cfg(feature = "middleware")]
pub use anyhow::Error as MiddlewareError;
#[cfg(feature = "middleware")]
pub use reqwest_middleware::ClientWithMiddleware as MiddlewareClient;

/// Wrapper over reqwest::Client or reqwest_middleware::ClientWithMiddleware
#[derive(Clone, Debug)]
pub enum Client {
    Vanilla(VanillaClient),
    #[cfg(feature = "middleware")]
    Middleware(MiddlewareClient),
}

impl From<VanillaClient> for Client {
    fn from(value: VanillaClient) -> Self {
        Client::Vanilla(value)
    }
}

#[cfg(feature = "middleware")]
impl From<MiddlewareClient> for Client {
    fn from(value: MiddlewareClient) -> Self {
        Client::Middleware(value)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    /// There was an error running some middleware
    #[cfg(feature = "middleware")]
    #[error("Middleware error: {0}")]
    Middleware(#[from] anyhow::Error),
    /// Error from the underlying reqwest client
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[cfg(feature = "middleware")]
impl From<reqwest_middleware::Error> for Error {
    fn from(value: reqwest_middleware::Error) -> Self {
        match value {
            reqwest_middleware::Error::Middleware(x) => Error::Middleware(x),
            reqwest_middleware::Error::Reqwest(x) => Error::Reqwest(x),
        }
    }
}

impl Client {
    /// See [`VanillaClient::get`]
    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    /// See [`VanillaClient::post`]
    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    /// See [`VanillaClient::put`]
    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    /// See [`VanillaClient::patch`]
    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// See [`VanillaClient::delete`]
    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// See [`VanillaClient::head`]
    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// See [`VanillaClient::request`]
    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        match self {
            Client::Vanilla(c) => RequestBuilder::Vanilla(c.request(method, url)),
            #[cfg(feature = "middleware")]
            Client::Middleware(c) => RequestBuilder::Middleware(c.request(method, url)),
        }
    }

    /// See [`VanillaClient::execute`]
    pub async fn execute(&self, req: Request) -> Result<Response, Error> {
        match self {
            Client::Vanilla(c) => c.execute(req).await.map_err(Into::into),
            #[cfg(feature = "middleware")]
            Client::Middleware(c) => {
                let mut ext = http::Extensions::new();
                c.execute_with_extensions(req, &mut ext)
                    .await
                    .map_err(Into::into)
            }
        }
    }

    /// Executes a request with initial [`Extensions`] if a MiddlewareClient.
    #[cfg(feature = "middleware")]
    pub async fn execute_with_extensions(
        &self,
        req: Request,
        ext: &mut http::Extensions,
    ) -> Result<Response, Error> {
        match self {
            Client::Vanilla(c) => c.execute(req).await.map_err(Into::into),
            Client::Middleware(c) => c
                .execute_with_extensions(req, ext)
                .await
                .map_err(Into::into),
        }
    }
}

/// This is a wrapper around [`reqwest::RequestBuilder`] and [`reqwest_middleware::RequestBuilder`] exposing the same API.
#[must_use = "RequestBuilder does nothing until you 'send' it"]
#[derive(Debug)]
pub enum RequestBuilder {
    Vanilla(reqwest::RequestBuilder),
    #[cfg(feature = "middleware")]
    Middleware(reqwest_middleware::RequestBuilder),
}

impl RequestBuilder {
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.header(key, value)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.header(key, value)),
        }
    }

    pub fn headers(self, headers: HeaderMap) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.headers(headers)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.headers(headers)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn version(self, version: reqwest::Version) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.version(version)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.version(version)),
        }
    }

    pub fn basic_auth<U, P>(self, username: U, password: Option<P>) -> Self
    where
        U: Display,
        P: Display,
    {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.basic_auth(username, password)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => {
                RequestBuilder::Middleware(c.basic_auth(username, password))
            }
        }
    }

    pub fn bearer_auth<T>(self, token: T) -> Self
    where
        T: Display,
    {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.bearer_auth(token)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.bearer_auth(token)),
        }
    }

    pub fn body<T: Into<Body>>(self, body: T) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.body(body)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.body(body)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn timeout(self, timeout: std::time::Duration) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.timeout(timeout)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.timeout(timeout)),
        }
    }

    #[cfg(feature = "multipart")]
    pub fn multipart(self, multipart: Form) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.multipart(multipart)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.multipart(multipart)),
        }
    }

    pub fn query<T: Serialize + ?Sized>(self, query: &T) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.query(query)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.query(query)),
        }
    }

    pub fn form<T: Serialize + ?Sized>(self, form: &T) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.form(form)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.form(form)),
        }
    }

    #[cfg(feature = "json")]
    pub fn json<T: Serialize + ?Sized>(self, json: &T) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.json(json)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.json(json)),
        }
    }

    pub fn build(self) -> reqwest::Result<Request> {
        match self {
            RequestBuilder::Vanilla(c) => c.build(),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => c.build(),
        }
    }

    /// Inserts the extension into this request builder (if middleware)
    #[cfg(feature = "middleware")]
    pub fn with_extension<T: Clone + Send + Sync + 'static>(self, extension: T) -> Self {
        match self {
            RequestBuilder::Middleware(c) => {
                RequestBuilder::Middleware(c.with_extension(extension))
            }
            c => c,
        }
    }

    /// Returns a mutable reference to the internal set of extensions for this request, or panics if not middleware
    #[cfg(feature = "middleware")]
    pub fn extensions(&mut self) -> &mut http::Extensions {
        match self {
            RequestBuilder::Vanilla(_) => panic!("attempted to get extensions of vanilla client"),
            RequestBuilder::Middleware(c) => c.extensions(),
        }
    }

    pub async fn send(self) -> Result<Response, Error> {
        match self {
            RequestBuilder::Vanilla(c) => c.send().await.map_err(Into::into),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => c.send().await.map_err(Into::into),
        }
    }

    /// Attempt to clone the RequestBuilder.
    ///
    /// `None` is returned if the RequestBuilder can not be cloned,
    /// i.e. if the request body is a stream.
    ///
    /// # Extensions
    /// Note that extensions are not preserved through cloning.
    pub fn try_clone(&self) -> Option<Self> {
        match self {
            RequestBuilder::Vanilla(c) => Some(RequestBuilder::Vanilla(c.try_clone()?)),
            #[cfg(feature = "middleware")]
            RequestBuilder::Middleware(c) => Some(RequestBuilder::Middleware(c.try_clone()?)),
        }
    }
}
