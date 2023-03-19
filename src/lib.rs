pub mod handler;
pub mod service;
pub mod server;
mod request;

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::{server::*, service::HttpStatus, request::Request};

    fn success() -> HttpStatus {
        HttpStatus::Success
    }

    fn query_book(book_no: usize) -> impl Into<HttpStatus> {
        println!("query book no.{book_no} .");

        HttpStatus::Success
    }

    fn post_bill(bill_no: usize, price: f32) -> impl Into<HttpStatus> {
        println!("bill_no: {bill_no} price: {price}");

        HttpStatus::Success
    }

    #[test]
    fn it_works() {
        let mut server = Server::new();
        
        server
            .get("/", success)
            .get("/book", query_book)
            .post("/bill", post_bill);

        let requests = {
            let req1 = Request::get("/", vec![]);
            let req2 = Request::get("/book", 10usize.to_le_bytes().into());
            let req3 = {
                let mut buf = vec![];
                buf.write(&20usize.to_le_bytes()).unwrap();
                buf.write(&1.2f32.to_le_bytes()).unwrap();
                Request::post("/bill", buf)
            };
            let err_request1 = Request::get("/404", vec![]);
            let err_request2 = Request::post("/404", vec![]);

            [req1, req2, req3, err_request1, err_request2]
        };

        for request in requests {
            let res = server.handle_request(request);
            println!("{res:?}");
        }
    }
}
