use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

/// Shared node state & actions.
#[derive(Debug)]
pub struct Node {
    /// Where this node is listening (e.g., "127.0.0.1:7001")
    pub port: String,
    /// Address of the next node in the ring; None until set
    pub next_port: RwLock<Option<String>>,
}

impl Node {
    pub fn new(port: String) -> Arc<Self> {
        Arc::new(Self { port, next_port: RwLock::new(None) })
    }

    pub async fn set_next(&self, addr: String) {
        *self.next_port.write().await = Some(addr);
    }

    pub async fn get_next(&self) -> Option<String> {
        self.next_port.read().await.clone()
    }

    /// Fire-and-forget forward of a RING command to the next node.
    pub async fn forward_ring(&self, hops: u32, msg: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(next) = self.get_next().await {
            let mut s = TcpStream::connect(&next).await?;
            let line = format!("RING {} {}\n", hops, msg);
            s.write_all(line.as_bytes()).await?;
        }
        Ok(())
    }
}
