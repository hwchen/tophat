//! This is the beginning of an example for database access.
//!
//! Still needs:
//! - db creation hardcoded in.
//! - web api from tophat.

mod pool;

use async_dup::Arc;
use futures_lite::{AsyncRead, AsyncWrite};
use http::Method;
use smol::Async;
use std::env;
use std::net::TcpListener;
use tophat::{
    server::{
        accept,
        glitch,
        router::{Router, RouterRequestExt},
        ResponseWriter, ResponseWritten,
    },
    Request,
};

use pool::{Pool, Manager};

fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // db setup
    let db_url = env::var("DATABASE_URL").expect("no db env var found");
    let mgr = Manager::new(&db_url)?;
    let pool = Pool::new(mgr, 16);

    // router setup
    let router = Router::build()
        .data(pool)
        //.at(Method::GET, "/:name", hello_user)
        .at(Method::GET, "/", get_user_count_by_country_and_org)
        .finish();

    let listener = Async::<TcpListener>::bind(([127,0,0,1],9999))?;

    smol::block_on(async {
        loop {
            let router = router.clone();

            let (stream, _) = listener.accept().await?;
            let stream = Arc::new(stream);

            let task = smol::spawn(async move {
                let serve = accept(stream, |req, resp_wtr| async {
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

async fn get_user_count_by_country_and_org<W>(req: Request, mut resp_wtr: ResponseWriter<W>) -> glitch::Result<ResponseWritten>
where
    W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
{
    let pool = req.data::<Pool>().unwrap();

    let client = pool.get().await?;

    let stmt = "SELECT country, COUNT(*) as count FROM users WHERE organization = 'Apple' GROUP BY country";
    let rows = client.query(stmt, &[]).await?;

    let body = rows.iter()
        .map(|r| {
            let country: &str = r.get(0);
            let count: i32 = r.get(1);
            format!("{}::{},", country, count)
        }).collect();

    resp_wtr.set_text(body);

    resp_wtr.send().await
}
