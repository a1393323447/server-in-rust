use crate::server::Path;

#[derive(Clone, Copy)]
pub enum RequestType {
    Get,
    Post,
}

pub struct Request {
    pub(crate) ty: RequestType,
    pub(crate) path: Path,
    pub(crate) payload: Vec<u8>,
}

impl Request {
    pub fn get(path: impl Into<Path>, payload: Vec<u8>) -> Request {
        Request { ty: RequestType::Get, path: path.into(), payload }
    }

    pub fn post(path: impl Into<Path>, payload: Vec<u8>) -> Request {
        Request { ty: RequestType::Post, path: path.into(), payload }
    }
}
