use crate::{NodeStatus, node::port_str};
use serde::Serialize;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, copy};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug)]
pub struct Gateway {
    /// Full addresses
    node_addrs: Vec<String>,
}

/// HTTP Response Struct
#[derive(Serialize)]
struct FileInfo {
    name: String,
    start: u16,
    size: u64,
}

impl Gateway {
    pub fn new(node_addrs: Vec<String>) -> Arc<Self> {
        Arc::new(Self { node_addrs })
    }

    /// Runs the main TCP server to listen for clients
    pub async fn run_server(self: Arc<Self>, listen_addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(&listen_addr).await?;
        tracing::info!(addr = %listen_addr, "Gateway listening (HTTP + TCP)");

        loop {
            let (client_stream, client_addr) = listener.accept().await?;
            let gateway_clone = Arc::clone(&self);

            tokio::spawn(async move {
                if let Err(e) = gateway_clone.handle_connection(client_stream).await {
                    tracing::warn!(client = %client_addr, error = ?e, "Gateway client error");
                }
            });
        }
    }

    /// An implementation of a protocol sniffer.
    async fn handle_connection(
        self: Arc<Self>,
        stream: TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // 1. Read the first line to sniff the protocol.
        let mut first_line = String::new();
        if let Err(e) = buf_reader.read_line(&mut first_line).await {
            tracing::debug!(error = ?e, "Client disconnected before sending data");
            return Ok(());
        }

        // 2. Check if the protocol is HTTP raw TCP
        if first_line.starts_with("GET /") {
            // Handle HTTP request
            tracing::debug!(line = %first_line.trim(), "Handling HTTP request");
            self.handle_http_request(&mut writer, &first_line).await?;
        } else {
            // Handle raw TCP
            tracing::debug!(line = %first_line.trim(), "Handling TCP proxy request");
            // Re-combine the stream to pass it to the proxy handler
            let stream = buf_reader.into_inner().reunite(writer)?;
            self.handle_tcp_proxy(stream, &first_line).await?;
        }
        Ok(())
    }

    // --- HTTP HANDLER ---

    async fn handle_http_request(
        self: Arc<Self>,
        writer: &mut (impl AsyncWrite + Unpin),
        first_line: &str,
    ) -> io::Result<()> {
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        let path = if parts.len() > 1 { parts[1] } else { "/" };

        match path {
            "/api/nodes" => match self.fetch_node_map().await {
                Ok(map) => Self::send_json_response(writer, &map).await,
                Err(e) => Self::send_error_response(writer, 500, &e.to_string()).await,
            },
            "/api/files" => match self.fetch_file_list().await {
                Ok(list) => Self::send_json_response(writer, &list).await,
                Err(e) => Self::send_error_response(writer, 500, &e.to_string()).await,
            },
            _ => Self::send_error_response(writer, 404, "Not Found").await,
        }
    }

    // --- TCP HANDLER ---

    async fn handle_tcp_proxy(
        self: Arc<Self>,
        mut client_stream: TcpStream,
        first_line: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 1. Find a live node
        let Some(addr) = self.find_alive_node().await else {
            client_stream
                .write_all(b"ERR no nodes available in the network\n")
                .await?;
            return Ok(());
        };

        // 2. Connect to that node
        let mut node_stream = match TcpStream::connect(&addr).await {
            Ok(stream) => stream,
            Err(e) => {
                client_stream
                    .write_all(b"ERR failed to connect to target node\n")
                    .await?;
                return Err(e.into());
            }
        };

        // 3. Send the line that was already read from the client
        node_stream.write_all(first_line.as_bytes()).await?;

        // 4. Proxy all remaining data in both directions
        copy_bidirectional_manual(&mut client_stream, &mut node_stream).await?;
        Ok(())
    }

    // --- API DATA FETCHERS ---

    /// Connects to the ring and sends `NETMAP GET`.
    async fn fetch_node_map(
        &self,
    ) -> Result<HashMap<String, NodeStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let mut stream = self.connect_to_ring().await?;
        stream.write_all(b"NETMAP GET\n").await?;

        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        let mut map = HashMap::new();

        while reader.read_line(&mut line).await? > 0 {
            if line.starts_with("OK") {
                break;
            }
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
        Ok(map)
    }

    /// Connects to the ring and sends `FILE LIST`.
    async fn fetch_file_list(
        &self,
    ) -> Result<Vec<FileInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let mut stream = self.connect_to_ring().await?;
        stream.write_all(b"FILE LIST\n").await?;

        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        let mut files = Vec::new();

        // Skip CSV header
        let _ = reader.read_line(&mut line).await?;
        line.clear();

        while reader.read_line(&mut line).await? > 0 {
            if line.trim().is_empty() {
                break;
            }
            let parts: Vec<&str> = line.trim().splitn(3, ',').collect();
            if parts.len() == 3 {
                // Handle CSV escaping
                let name = parts[0].trim_matches('\"');

                files.push(FileInfo {
                    name: name.to_string(),
                    start: parts[1].parse().unwrap_or(0),
                    size: parts[2].parse().unwrap_or(0),
                });
            }
            line.clear();
        }
        Ok(files)
    }

    // --- TCP HELPERS ---

    /// Tries all node addresses and returns a stream to the first one that connects.
    async fn connect_to_ring(&self) -> Result<TcpStream, Box<dyn std::error::Error + Send + Sync>> {
        for addr in &self.node_addrs {
            if let Ok(stream) = TcpStream::connect(addr).await {
                return Ok(stream);
            }
        }
        Err("Could not connect to any node in the ring".into())
    }

    /// Finds the first 'Alive' node.
    async fn find_alive_node(&self) -> Option<String> {
        for addr in &self.node_addrs {
            if TcpStream::connect(addr).await.is_ok() {
                return Some(addr.clone());
            }
        }
        None
    }

    // --- HTTP HELPERS ---

    async fn send_json_response<T: Serialize>(
        writer: &mut (impl AsyncWrite + Unpin),
        data: T,
    ) -> io::Result<()> {
        let json = serde_json::to_string(&data).unwrap_or("{}".to_string());
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            json.len(),
            json
        );
        writer.write_all(response.as_bytes()).await
    }

    async fn send_error_response(
        writer: &mut (impl AsyncWrite + Unpin),
        status: u16,
        message: &str,
    ) -> io::Result<()> {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            message,
            message.len(),
            message
        );
        writer.write_all(response.as_bytes()).await
    }
}

// Manual copy_bidirectional that works with a pre-split stream
async fn copy_bidirectional_manual(
    client: &mut TcpStream,
    server: &mut TcpStream,
) -> io::Result<()> {
    let (mut client_read, mut client_write) = client.split();
    let (mut server_read, mut server_write) = server.split();

    tokio::select! {
        res = copy(&mut client_read, &mut server_write) => res,
        res = copy(&mut server_read, &mut client_write) => res,
    }?;
    Ok(())
}
