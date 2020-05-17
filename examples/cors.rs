//! Cors example
//!
//! Careful, it's not completely automatic middleware, you currently have to use a switch statement
//! to get the correct early-return behavior.

use futures_util::io::{AsyncRead, AsyncWrite};
use http::Method;
use smol::{Async, Task};
use std::net::TcpListener;
use piper::Arc;
use tophat::{
    server::{
        accept,
        cors::{Cors, Validated},
        glitch::Result,
        router::{Router, RouterRequestExt},
        ResponseWriter,
        ResponseWritten,
    },
    Request,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let cors = Cors::new()
        .allow_origin("http://example.com")
        .allow_methods(vec!["GET", "POST", "DELETE"])
        .finish();

    let router = Router::build()
        .data("Data from datastore")
        .at(Method::GET, "/:name", hello_user)
        .finish();

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let cors = cors.clone();
            let router = router.clone();

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(stream, |req, mut resp_wtr| async {
                    // TODO better early return handling for middlware?
                    match cors.validate(&req, &mut resp_wtr) {
                        Validated::Preflight | Validated::Invalid => {
                            return resp_wtr.send().await;
                        },
                        Validated::Simple | Validated::NotCors => (),
                    }

                    // back to routing here
                    let res = router.route(req, resp_wtr).await;
                    res
                }).await;

                if let Err(err) = serve {
                    eprintln!("Error: {}", err);
                }
            });

            task.detach();
        }
    })
}

async fn hello_user< W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
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
