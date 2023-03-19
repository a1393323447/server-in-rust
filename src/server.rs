use std::collections::HashMap;
use std::fmt::Display;

use crate::request::{Request, RequestType};
use crate::service::{BoxedService, HttpStatus, FromPayload, Payload};
use crate::handler::{Factory, Handler};

#[derive(Default)]
pub struct Server {
    get: HashMap<Path, BoxedService>,
    post: HashMap<Path, BoxedService>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<P, F, A, R>(&mut self, path: P, f: F) -> &mut Self 
    where
        P: Into<Path>,
        A: FromPayload + 'static,
        R: Into<HttpStatus> + 'static, 
        F: Factory<A, R> + 'static,
    {
        let handler = Handler::new(f);
        self.get.insert(path.into(), BoxedService::from_handler(handler));

        self
    }

    pub fn post<P, F, A, R>(&mut self, path: P, f: F) -> &mut Self 
    where
        P: Into<Path>,
        A: FromPayload + 'static,
        R: Into<HttpStatus> + 'static, 
        F: Factory<A, R> + 'static,
    {
        let handler = Handler::new(f);
        self.post.insert(path.into(), BoxedService::from_handler(handler));

        self
    }

    pub fn handle_request(&self, request: Request) -> Result<HttpStatus, String> {
        let service = match request.ty {
            RequestType::Get => match self.get.get(&request.path) {
                Some(s) => s,
                None => return Err(format!("missing get handler for path {}", request.path)), 
            },
            RequestType::Post => match self.post.get(&request.path) {
                Some(s) => s,
                None => return Err(format!("missing post handler for path {}", request.path)), 
            },
        };

        let payload = Payload::from_bytes(&request.payload);
        Ok(service.handle(payload).into())
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Path {
    p: String,
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.p)
    }
}

impl<T> From<T> for Path 
    where T: Into<String>
{
    fn from(value: T) -> Self {
        Path { p: value.into() }
    }
}