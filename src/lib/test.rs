use tokio::net::TcpListener;

use super::prelude::*;

#[derive(Debug)]
pub struct Server {
    l: TcpListener,
}

impl Server {
    pub async fn new() -> Result<Self> {
        let l = TcpListener::bind("127.0.0.1:0").await?;
    }
}
