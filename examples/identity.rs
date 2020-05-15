use futures_util::io::{AsyncRead, AsyncWrite};
use http::Method;
use smol::{Async, Task};
use std::net::TcpListener;
use std::time::Duration;
use piper::Arc;
use tophat::server::{
    accept,
    identity::Identity,
    reply,
    router::{Router, RouterRequestExt},
    Request,
    ResponseWriter,
    ResponseWritten,
    Result,
};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let identity = Identity::new("secret_server_key")
        .cookie_name("jwt")
        .cookie_secure(false) // necessary because example not https
        .issuer("tophat")
        .expiration_time(Duration::from_secs(30))
        .finish();

    let router = Router::build()
        .data(identity)
        .at(Method::GET, "/login/:user", login_user)
        .at(Method::GET, "/logout", logout_user)
        .at(Method::GET, "/", hello_user)
        .finish();

    let listener = Async::<TcpListener>::bind("127.0.0.1:9999")?;

    smol::run(async {
        loop {
            let router = router.clone();

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = Task::spawn(async move {
                let serve = accept(stream, |req, resp_wtr| async {
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

async fn login_user<W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let identity = req.data::<Identity>().unwrap();
    let user = req.get_param("user").unwrap();

    // Here, we'll just assume that user is valid. This will usually be a call to the db to check
    // against hashed password.

    // Since user is valid, we'll set a cookie with the jwt token
    let mut resp = reply::code(200).unwrap();
    identity.set_auth_token(user, &mut resp);

    println!("Login req headers{:?}", req.headers());
    println!("Login res headers{:?}", resp.headers());

    *resp_wtr.response_mut() = resp;
    resp_wtr.send().await
}

async fn logout_user<W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    // Since we're using jwt tokens, we don't need to do a check on some session store to remove
    // the session; just send the "forget" cookie.

    let identity = req.data::<Identity>().unwrap();

    let mut resp = reply::code(200).unwrap();
    identity.forget(&mut resp);

    println!("Logout req headers{:?}", req.headers());
    println!("Logout res headers{:?}", resp.headers());

    *resp_wtr.response_mut() = resp;
    resp_wtr.send().await
}

// Says hello to user based on user login name
async fn hello_user<W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> Result<ResponseWritten>
    where W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let identity = req.data::<Identity>().unwrap();

    println!("Hello req headers{:?}", req.headers());

    let user = match identity.authorized_user(&req) {
        Some(u) => u,
        None => {
            resp_wtr.set_code(400);
            return resp_wtr.send().await;
        },
    };

    resp_wtr.set_text(format!("Hello {}", user));
    resp_wtr.send().await
}
