use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::io::{self, AsyncBufReadExt};

#[derive(Clone)]
pub struct Peer {
  socket: Arc<UdpSocket>
}

impl Peer {
  pub async fn new (bind_addr: &str) -> io::Result<Peer> {
    let socket = UdpSocket::bind(bind_addr).await?;

    println!("Listening on: {}", bind_addr);

    Ok(Peer { socket: Arc::new(socket) })
  }

  pub async fn send_message(&self, msg: &str, target_addr: &SocketAddr) -> io::Result<()> {
    self.socket.send_to(msg.as_bytes(), target_addr).await?;
    Ok(())
  }

  pub async fn receive_message(&self) -> io::Result<(String, SocketAddr)> {
    let mut buf = [0u8; 1024]; // buffer

    let (len, addr) = self.socket.recv_from(&mut buf).await?;
    let msg = String::from_utf8_lossy(&buf[..len]).to_string();
    
    Ok((msg, addr))
  }
}
