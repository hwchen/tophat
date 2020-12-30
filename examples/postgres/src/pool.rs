//! Pool for db connections
//!
//! Based on deadpool-postgres. For more config and features (like prepared statements), use its
//! implementation as a starting point.

use anyhow::Context as _;
use async_trait::async_trait;
use smol::Async;
use std::net::{TcpStream, ToSocketAddrs};
use tokio_postgres::{tls::NoTls, Client};
use tokio_util::compat::FuturesAsyncWriteCompatExt;
use tracing::debug;

pub(crate) type Pool = deadpool::managed::Pool<Client, Error>;
pub (crate) type RecycleError = deadpool::managed::RecycleError<Error>;

pub (crate) struct Manager {
    pg_config: tokio_postgres::config::Config,
    socket_addr: std::net::SocketAddr,
}

impl Manager {
    pub(crate) fn new(db_url: &str) -> Result<Self, anyhow::Error> {
        let pg_config = db_url.parse()?;

        let db_url: url::Url = db_url.parse()?;
        // Figure out the host and the port.
        let host = db_url.host().context("cannot parse host")?.to_string();
        let port = db_url
            .port()
            .unwrap_or(5432);

        // Connect to the host.
        let socket_addr = (host.as_str(), port)
            .to_socket_addrs()?
            .next()
            .context("cannot resolve address")?;

        Ok(Self {
            pg_config,
            socket_addr,
        })
    }
}

#[async_trait]
impl deadpool::managed::Manager<Client, Error> for Manager {
    async fn create(&self) -> Result<Client, Error> {
        debug!("Pool: create client");
        let stream = Async::<TcpStream>::connect(self.socket_addr).await?;
        let stream = stream.compat_write();
        let (client, connection) = self.pg_config.connect_raw(stream, NoTls).await?;
        smol::spawn(connection).detach();

        Ok(client)
    }

    async fn recycle(&self, client: &mut Client) -> Result<(), RecycleError> {
        debug!("Pool: recycle client");
        if client.is_closed() {
            return Err(RecycleError::Message("Connection closed".to_string()));
        }
        // "fast" recycling method from doesn't run a query
        //client.simple_query(None).await
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("postgres error")]
    Postgres(#[from] tokio_postgres::Error),
}
