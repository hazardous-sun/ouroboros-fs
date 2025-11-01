use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::{oneshot, RwLock},
};

/// Shared node state & actions.
///
/// - `next_port`: configured next hop (if any).
/// - WALK uses a token->oneshot table at the start node.
/// - FILE push also uses token->oneshot at the start node (to confirm loop).
#[derive(Debug)]
pub struct Node {
    /// Where this node is listening (e.g., "127.0.0.1:7001")
    pub port: String,

    /// Address of the next node in the ring; None until set via SET_NEXT
    pub next_port: RwLock<Option<String>>,

    // WALK pending acks (start node only)
    pending_walks: RwLock<HashMap<String, oneshot::Sender<String>>>,
    walk_counter: AtomicU64,

    // FILE pending acks (start node only)
    pending_files: RwLock<HashMap<String, oneshot::Sender<()>>>,
    file_counter: AtomicU64,
}

impl Node {
    pub fn new(port: String) -> Arc<Self> {
        Arc::new(Self {
            port,
            next_port: RwLock::new(None),
            pending_walks: RwLock::new(HashMap::new()),
            walk_counter: AtomicU64::new(1),
            pending_files: RwLock::new(HashMap::new()),
            file_counter: AtomicU64::new(1),
        })
    }

    pub async fn set_next(&self, addr: String) {
        *self.next_port.write().await = Some(addr);
    }

    pub async fn get_next(&self) -> Option<String> {
        self.next_port.read().await.clone()
    }

    /* ---------------- RING forwarding ---------------- */

    pub async fn forward_ring(
        &self,
        ttl: u32,
        msg: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(next) = self.get_next().await {
            let mut s = TcpStream::connect(&next).await?;
            let line = format!("RING {} {}\n", ttl, msg);
            s.write_all(line.as_bytes()).await?;
        }
        Ok(())
    }

    /* ---------------- WALK helpers ---------------- */

    fn next_token(&self) -> String {
        let n = self.walk_counter.fetch_add(1, Ordering::Relaxed);
        format!("{}-{}", self.port, n)
    }

    pub fn make_walk_token(&self) -> String {
        self.next_token()
    }

    pub async fn register_walk(&self, token: &str) -> oneshot::Receiver<String> {
        let (tx, rx) = oneshot::channel();
        self.pending_walks.write().await.insert(token.to_string(), tx);
        rx
    }

    pub async fn finish_walk(&self, token: &str, history: String) -> bool {
        if let Some(tx) = self.pending_walks.write().await.remove(token) {
            let _ = tx.send(history);
            true
        } else {
            false
        }
    }

    pub async fn forward_walk_hop(
        &self,
        token: &str,
        start_addr: &str,
        history: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(next) = self.get_next().await {
            let mut s = TcpStream::connect(&next).await?;
            let line = format!("WALK HOP {} {} {}\n", token, start_addr, history);
            s.write_all(line.as_bytes()).await?;
        }
        Ok(())
    }

    pub async fn send_walk_done(
        &self,
        start_addr: &str,
        token: &str,
        history: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut s = TcpStream::connect(start_addr).await?;
        let line = format!("WALK DONE {} {}\n", token, history);
        s.write_all(line.as_bytes()).await?;
        Ok(())
    }

    /* ---------------- FILE helpers ---------------- */

    fn next_file_token(&self) -> String {
        let n = self.file_counter.fetch_add(1, Ordering::Relaxed);
        format!("file-{}-{}", self.port, n)
    }

    pub fn make_file_token(&self) -> String {
        self.next_file_token()
    }

    pub async fn register_file(&self, token: &str) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.pending_files.write().await.insert(token.to_string(), tx);
        rx
    }

    pub async fn finish_file(&self, token: &str) -> bool {
        if let Some(tx) = self.pending_files.write().await.remove(token) {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }

    pub async fn forward_file_hop(
        &self,
        token: &str,
        start_addr: &str,
        size: u64,
        name: &str,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(next) = self.get_next().await {
            let mut s = TcpStream::connect(&next).await?;
            let header = format!("FILE HOP {} {} {} {}\n", token, start_addr, size, name);
            s.write_all(header.as_bytes()).await?;
            s.write_all(data).await?;
        }
        Ok(())
    }
}

/* ---------- WALK utility ---------- */

pub fn port_str(addr: &str) -> &str {
    addr.rsplit(':').next().unwrap_or(addr)
}

pub fn append_edge(mut history: String, from_addr: &str, to_addr: &str) -> String {
    let from = port_str(from_addr);
    let to = port_str(to_addr);
    let edge = format!("{from}->{to}");
    if history.is_empty() {
        edge
    } else {
        history.push(';');
        history.push_str(&edge);
        history
    }
}

impl Node {
    pub async fn first_walk_history(&self) -> Option<String> {
        let next = self.get_next().await?;
        Some(append_edge(String::new(), &self.port, &next))
    }
}
