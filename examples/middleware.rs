//! Example of "middleware" with simple cors.
//!
//! It's kind of a do-it-yourself middleware, there's not formal framework for it. It should be
//! easy enough to plug in.

use async_dup::Arc;
use futures_util::io::{AsyncRead, AsyncWrite};
use http::Method;
use smol::Async;
use std::net::TcpListener;
use tophat::{
    server::{
        accept,
        glitch::Result,
        router::{Router, RouterRequestExt},
        ResponseWriter, ResponseWritten,
    },
    Request,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cors = Arc::new(Cors {
        allow_origin: "*".to_owned(),
    });

    let router = Router::build()
        .data("Data from datastore")
        .at(Method::GET, "/:name", hello_user)
        .finish();

    let listener = Async::<TcpListener>::bind(([127,0,0,1],9999))?;

    smol::block_on(async {
        loop {
            let cors = cors.clone();
            let router = router.clone();

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = smol::spawn(async move {
                let serve = accept(stream, |req, mut resp_wtr| async {
                    // Do the middleware thing here
                    // Cors preflight would require something like
                    // ```
                    // if cors.preflight(&req, &mut resp_wtr) {
                    //     return resp_wtr.send();
                    // }
                    // ```
                    cors.simple_cors(&req, &mut resp_wtr);

                    // back to routing here
                    let res = router.route(req, resp_wtr).await;
                    res
                })
                .await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }
            });

            task.detach();
        }
    })
}

async fn hello_user<W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    //smol::Timer::after(std::time::Duration::from_secs(5)).await;

    let mut resp_body = format!("Hello, ");

    // add params to body string
    if let Some(params) = req.params() {
        for (k, v) in params {
            resp_body.push_str(&format!("{} = {}", k, v));
        }
    }

    // add data to body string
    if let Some(data_string) = req.data::<&str>() {
        resp_body.push_str(&format!(" and {}", *data_string));
    }

    resp_wtr.set_body(resp_body.into());

    resp_wtr.send().await
}

struct Cors {
    allow_origin: String,
}

impl Cors {
    // Sets the Access Control Header on the Response of a Responsewriter, if Origin in Request is
    // set.
    //
    // No preflight.
    //
    // Unless the user changes the header in the endpoint, the header should be sent to the client.
    fn simple_cors<W>(&self, req: &Request, resp_wtr: &mut ResponseWriter<W>)
    where
        W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        if req.headers().get("Origin").is_some() {
            resp_wtr.insert_header(
                "Access-Control-Allow-Origin",
                self.allow_origin.parse().unwrap(),
            );
        }
    }
}
