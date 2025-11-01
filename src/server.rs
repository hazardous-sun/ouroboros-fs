use std::{error, sync::Arc};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    node::Node,
    protocol::{self, Command},
};

type AnyErr = Box<dyn error::Error + Send + Sync>;

/// Run the TCP server and handle connections.
pub async fn run(bind_addr: &str) -> Result<(), AnyErr> {
    let listener = TcpListener::bind(bind_addr).await?;
    let local = listener.local_addr()?;
    let node = Node::new(local.to_string());

    println!("node listening on {}", node.port);

    loop {
        let (stream, peer) = listener.accept().await?;
        let node = Arc::clone(&node);
        tokio::spawn(async move {
            if let Err(e) = handle_client(node, stream).await {
                eprintln!("client {peer}: error: {e}");
            }
        });
    }
}

async fn handle_client(node: Arc<Node>, stream: TcpStream) -> Result<(), AnyErr> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }

        match protocol::parse_line(&line) {
            Ok(cmd) => dispatch_command(&node, &mut writer, cmd).await?,
            Err(e) => handle_error(&mut writer, e).await?,
        }
    }
    Ok(())
}

async fn dispatch_command<W: AsyncWrite + Unpin>(
    node: &Node,
    writer: &mut W,
    cmd: Command,
) -> Result<(), AnyErr> {
    match cmd {
        Command::SetNext(addr) => handle_set_next(node, writer, addr).await,
        Command::Get => handle_get(node, writer).await,
        Command::Ring { hops, msg } => handle_ring(node, writer, hops, msg).await,
    }
}

/* --- Command handlers --- */

async fn handle_set_next<W: AsyncWrite + Unpin>(
    node: &Node,
    writer: &mut W,
    addr: String,
) -> Result<(), AnyErr> {
    node.set_next(addr.clone()).await;
    writer
        .write_all(format!("OK next={}\n", addr).as_bytes())
        .await?;
    Ok(())
}

async fn handle_get<W: AsyncWrite + Unpin>(
    node: &Node,
    writer: &mut W,
) -> Result<(), AnyErr> {
    let next = node.get_next().await;
    writer
        .write_all(
            format!(
                "PORT {}\nNEXT {}\n",
                node.port,
                next.as_deref().unwrap_or("<unset>")
            )
                .as_bytes(),
        )
        .await?;
    Ok(())
}

async fn handle_ring<W: AsyncWrite + Unpin>(
    node: &Node,
    writer: &mut W,
    mut hops: u32,
    msg: String,
) -> Result<(), AnyErr> {
    println!("[{}] RING(hops={}) msg: {}", node.port, hops, msg);
    if hops > 0 {
        hops -= 1;
        if let Err(e) = node.forward_ring(hops, &msg).await {
            eprintln!("[{}] forward error: {}", node.port, e);
        }
    }
    writer.write_all(b"OK\n").await?;
    Ok(())
}

async fn handle_error<W: AsyncWrite + Unpin>(
    writer: &mut W,
    err: String,
) -> Result<(), AnyErr> {
    writer
        .write_all(format!("ERR {}\n", err).as_bytes())
        .await?;
    Ok(())
}
