use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite};
use tokio_util::codec::LengthDelimitedCodec;

use crate::config::config::MeshConfig;
use crate::service::mysql::MySQLService;

#[async_trait]
pub trait ServiceHandler {
    async fn handle(&self, mut socket: TcpStream);
}

#[async_trait]
pub trait Service {
    async fn serve(&self) -> std::result::Result<(), Box<dyn std::error::Error>>;
}

pub trait ServiceCodec {
    fn write_frame<T: AsyncWrite>(&self, io: T) -> FramedWrite<T, LengthDelimitedCodec>;
    fn read_frame<T: AsyncRead>(&self, io: T) -> FramedRead<T, LengthDelimitedCodec>;
}

pub fn new_service() -> Box<dyn Service> {
    Box::new(MySQLService {})
}