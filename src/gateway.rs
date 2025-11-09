use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::time::sleep;
use crate::NodeStatus;

type NodeStatusMap = Arc<RwLock<HashMap<String, NodeStatus>>>;

#[derive(Debug)]
pub struct Gateway {
    /// Full addresses
    node_addrs: Vec<String>,
    /// The shared, polled network status
    status_map: NodeStatusMap,
}

impl Gateway {
    pub fn new(node_addrs: Vec<String>) -> Arc<Self> {
        Arc::new(Self {
            node_addrs,
            status_map: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Runs the main TCP server to listen for client commands
    pub async fn run_server(self: Arc<Self>, listen_addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(&listen_addr).await?;
        tracing::info!(addr = %listen_addr, "DNS Gateway listening");

        loop {
            let (client_stream, client_addr) = listener.accept().await?;
            tracing::debug!(client = %client_addr, "Gateway accepted new client");

            let gateway_clone = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = gateway_clone.handle_client(client_stream).await {
                    tracing::warn!(client = %client_addr, error = ?e, "Gateway client error");
                }
            });
        }
    }

    /// Runs a background loop to poll the network status
    pub async fn run_polling_loop(self: Arc<Self>, interval: Duration) {
        tracing::info!(poll_interval = ?interval, "Gateway poller starting");
        loop {
            sleep(interval).await;

            match self.fetch_network_map().await {
                Ok(map) => {
                    let mut w_lock = self.status_map.write().await;
                    *w_lock = map;
                    tracing::debug!("Gateway polled and updated network map");
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "Gateway poll failed to get network map");
                }
            }
        }
    }

    /// Handles a single client, finds a node, and proxies the connection
    async fn handle_client(self: Arc<Self>, mut client_stream: TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 1. Find the first one that is 'Alive'
        let target_addr = self.find_alive_node().await;

        let Some(addr) = target_addr else {
            client_stream.write_all(b"ERR no nodes available in the network\n").await?;
            return Ok(());
        };

        // 2. Connect to that node
        let mut node_stream = match TcpStream::connect(&addr).await {
            Ok(stream) => stream,
            Err(e) => {
                client_stream.write_all(b"ERR failed to connect to target node\n").await?;
                return Err(e.into());
            }
        };

        tracing::debug!(target_node = %addr, "Gateway proxying client");

        // 3. Proxy all data in both directions
        tokio::io::copy_bidirectional(&mut client_stream, &mut node_stream).await?;

        tracing::debug!(target_node = %addr, "Gateway proxy finished");
        Ok(())
    }

    /// Finds the first 'Alive' node from the status map
    async fn find_alive_node(&self) -> Option<String> {
        let r_lock = self.status_map.read().await;
        // Find the first entry that is "Alive"
        let (port_str, _) = r_lock.iter().find(|&(_, &status)| status == NodeStatus::Alive)?;

        // We need to find the full address from the node_addrs list,
        // because the map only stores the port (e.g., "7000")
        self.node_addrs.iter().find(|addr| crate::node::port_str(addr) == port_str).cloned()
    }

    /// Connects to one node to get the status of all nodes
    async fn fetch_network_map(&self) -> Result<HashMap<String, NodeStatus>, Box<dyn std::error::Error + Send + Sync>> {
        // Try any of the known nodes, the first one that answers is fine.
        for addr in &self.node_addrs {
            match TcpStream::connect(addr).await {
                Ok(mut stream) => {
                    stream.write_all(b"NETMAP GET\n").await?;
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();
                    let mut map = HashMap::new();

                    while reader.read_line(&mut line).await? > 0 {
                        if line.starts_with("OK") { break; }
                        if let Some((port, status_str)) = line.trim().split_once('=') {
                            let status = if status_str.eq_ignore_ascii_case("Dead") {
                                NodeStatus::Dead
                            } else {
                                NodeStatus::Alive
                            };
                            map.insert(port.to_string(), status);
                        }
                        line.clear();
                    }
                    return Ok(map);
                }
                Err(_) => continue,
            }
        }
        Err("Could not connect to any node to get network map".into())
    }
}
