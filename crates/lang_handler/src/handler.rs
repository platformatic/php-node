use crate::{Request, Response};

pub trait Handler {
    type Error;

    fn handle(&self, request: Request) -> Result<Response, Self::Error>;
}
