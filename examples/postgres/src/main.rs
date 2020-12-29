//! TODO double check... I'm using smol here, but maybe sqlx is just starting up async-std runtime
//! and using that instead.
//!
//! This is the beginning of an example for database access.
//!
//! Still needs:
//! - db creation hardcoded in.
//! - web api from tophat.

use anyhow::Context as _;
use async_trait::async_trait;
use tokio_postgres::{tls::NoTls, Client};
use tokio_util::compat::FuturesAsyncWriteCompatExt;
use smol::Async;
use std::env;
use std::net::{TcpStream, ToSocketAddrs};

fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    smol::block_on(async move {
        let db_url = env::var("DATABASE_URL").expect("no db env var found");
        let mgr = Manager::new(&db_url).await?;
        let pool = Pool::new(mgr, 16);

        let client = pool.get().await.expect("not sure why this one can't Into anyhow::Error");

        let stmt = "SELECT country, COUNT(*) as count FROM users WHERE organization = 'Apple' GROUP BY country";
        let rows = client.query(stmt, &[]).await?;

        for row in rows {
            println!("{:?}", row);
        }

        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}

type Pool = deadpool::managed::Pool<Client, anyhow::Error>;
type RecycleError = deadpool::managed::RecycleError<anyhow::Error>;

struct Manager {
    pg_config: tokio_postgres::config::Config,
    socket_addr: std::net::SocketAddr,
}

impl Manager {
    async fn new(db_url: &str) -> Result<Self, anyhow::Error> {
        let pg_config = db_url.parse()?;

        let db_url: url::Url = db_url.parse()?;
        // Figure out the host and the port.
        let host = db_url.host().context("cannot parse host")?.to_string();
        let port = db_url
            .port()
            .unwrap_or(5432);

        // Connect to the host.
        let socket_addr = {
            let host = host.clone();
            smol::unblock(move || (host.as_str(), port).to_socket_addrs())
                .await?
                .next()
                .context("cannot resolve address")?
        };

        Ok(Self {
            pg_config,
            socket_addr,
        })
    }
}

#[async_trait]
impl deadpool::managed::Manager<Client, anyhow::Error> for Manager {
    async fn create(&self) -> Result<Client, anyhow::Error> {
        let stream = Async::<TcpStream>::connect(self.socket_addr).await?;

        let stream = stream.compat_write();
        let (client, connection) = self.pg_config.connect_raw(stream, NoTls).await?;
        let conn_task = smol::spawn(connection);
        conn_task.detach();

        Ok(client)
    }

    async fn recycle(&self, client: &mut Client) -> deadpool::managed::RecycleResult<anyhow::Error> {
        if client.is_closed() {
            return Err(RecycleError::Message("Connection closed".to_string()));
        }
        // "fast" recycling method from doesn't run a query
        //client.simple_query(None).await
        Ok(())
    }
}
