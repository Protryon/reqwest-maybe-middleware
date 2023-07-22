use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::multipart::Form;
use reqwest::{Body, Client as VanillaClient, IntoUrl, Method, Request, Response};
pub use reqwest_middleware::{ClientWithMiddleware as MiddlewareClient, Result};
use serde::Serialize;
use std::convert::TryFrom;
use std::fmt::Display;
use task_local_extensions::Extensions;

/// Wrapper over reqwest::Client or reqwest_middleware::ClientWithMiddleware
#[derive(Clone, Debug)]
pub enum Client {
    Vanilla(VanillaClient),
    Middleware(MiddlewareClient),
}

impl From<VanillaClient> for Client {
    fn from(value: VanillaClient) -> Self {
        Client::Vanilla(value)
    }
}

impl From<MiddlewareClient> for Client {
    fn from(value: MiddlewareClient) -> Self {
        Client::Middleware(value)
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
            Client::Middleware(c) => RequestBuilder::Middleware(c.request(method, url)),
        }
    }

    /// See [`VanillaClient::execute`]
    pub async fn execute(&self, req: Request) -> Result<Response> {
        let mut ext = Extensions::new();
        self.execute_with_extensions(req, &mut ext).await
    }

    /// Executes a request with initial [`Extensions`] if a MiddlewareClient.
    pub async fn execute_with_extensions(
        &self,
        req: Request,
        ext: &mut Extensions,
    ) -> Result<Response> {
        match self {
            Client::Vanilla(c) => c.execute(req).await.map_err(Into::into),
            Client::Middleware(c) => c.execute_with_extensions(req, ext).await,
        }
    }
}

/// This is a wrapper around [`reqwest::RequestBuilder`] and [`reqwest_middleware::RequestBuilder`] exposing the same API.
#[must_use = "RequestBuilder does nothing until you 'send' it"]
#[derive(Debug)]
pub enum RequestBuilder {
    Vanilla(reqwest::RequestBuilder),
    Middleware(reqwest_middleware::RequestBuilder),
}

impl RequestBuilder {
    fn map(
        self,
        map_vanilla: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
        map_middleware: impl FnOnce(
            reqwest_middleware::RequestBuilder,
        ) -> reqwest_middleware::RequestBuilder,
    ) -> Self {
        match self {
            Self::Vanilla(c) => Self::Vanilla(map_vanilla(c)),
            Self::Middleware(c) => Self::Middleware(map_middleware(c)),
        }
    }
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
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.header(key, value)),
        }
    }

    pub fn headers(self, headers: HeaderMap) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.headers(headers)),
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.headers(headers)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn version(self, version: reqwest::Version) -> Self {
        self.map(|c| c.version(version), |c| c.version(version))
    }

    pub fn basic_auth<U, P>(self, username: U, password: Option<P>) -> Self
    where
        U: Display,
        P: Display,
    {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.basic_auth(username, password)),
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
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.bearer_auth(token)),
        }
    }

    pub fn body<T: Into<Body>>(self, body: T) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.body(body)),
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.body(body)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn timeout(self, timeout: std::time::Duration) -> Self {
        self.map(|c| c.timeout(timeout), |c| c.timeout(timeout))
    }

    pub fn multipart(self, multipart: Form) -> Self {
        match self {
            RequestBuilder::Vanilla(c) => RequestBuilder::Vanilla(c.multipart(multipart)),
            RequestBuilder::Middleware(c) => RequestBuilder::Middleware(c.multipart(multipart)),
        }
    }

    pub fn query<T: Serialize + ?Sized>(self, query: &T) -> Self {
        self.map(|c| c.query(query), |c| c.query(query))
    }

    pub fn form<T: Serialize + ?Sized>(self, form: &T) -> Self {
        self.map(|c| c.form(form), |c| c.form(form))
    }

    pub fn json<T: Serialize + ?Sized>(self, json: &T) -> Self {
        self.map(|c| c.json(json), |c| c.json(json))
    }

    pub fn build(self) -> reqwest::Result<Request> {
        match self {
            RequestBuilder::Vanilla(c) => c.build(),
            RequestBuilder::Middleware(c) => c.build(),
        }
    }

    /// Inserts the extension into this request builder (if middleware)
    pub fn with_extension<T: Send + Sync + 'static>(self, extension: T) -> Self {
        match self {
            RequestBuilder::Middleware(c) => {
                RequestBuilder::Middleware(c.with_extension(extension))
            }
            c => c,
        }
    }

    /// Returns a mutable reference to the internal set of extensions for this request, or panics if not middleware
    pub fn extensions(&mut self) -> &mut Extensions {
        match self {
            RequestBuilder::Vanilla(_) => panic!("attempted to get extensions of vanilla client"),
            RequestBuilder::Middleware(c) => c.extensions(),
        }
    }

    pub async fn send(self) -> Result<Response> {
        match self {
            RequestBuilder::Vanilla(c) => c.send().await.map_err(Into::into),
            RequestBuilder::Middleware(c) => c.send().await,
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
            RequestBuilder::Middleware(c) => Some(RequestBuilder::Middleware(c.try_clone()?)),
        }
    }
}
