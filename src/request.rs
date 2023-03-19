use std::fmt::Display;

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