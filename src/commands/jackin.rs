use std::net::SocketAddr;
use tokio::task;

use crate::masp::receiver::MaspReceiver;
use crate::masp::sender::MaspSender;
use crate::video;

pub async fn run (port: u16, mut address: SocketAddr) -> Result<(), Box<dyn std::error::Error>>{
  let local_addr_str = "0.0.0.0";
  // SENDER will be always binded to the given port + 1
  let mut local_addr = SocketAddr::new(local_addr_str.parse()?, port + 1);
  let mut masp_sender = MaspSender::new(
    local_addr.clone(),
    address.clone()
  ).await?;

  // setting RECIEVER socket on given port
  local_addr.set_port(port);
  // remote SENDER also binded to 1 port further
  address.set_port(address.port() + 1);
  let mut masp_reciever = MaspReceiver::new(
    local_addr,
    Some(address.clone())
  ).await?;

  // UDP hole punching
  masp_sender.punch_hole(
    address.port(),
    address.port() + 1
  ).await?;

  // waiting for handshake to complete
  masp_sender.init_handshake().await?;

  // Start acknowledgment handling in a background task
  let ack_handler = {
    let sender_clone = masp_sender.clone();
    
    task::spawn(async move {
      sender_clone.handle_acknowledgments().await.unwrap();
    })
  };

  // Start retransmission handling in a background task
  let retransmitter = {
    let sender_clone = masp_sender.clone();
    
    task::spawn(async move {
      sender_clone.retransmit_unacknowledged().await;
    })
  };

  let video_stream = {
    let sender_clone = masp_sender.clone();

    task::spawn(async move {
      video::stream::run(sender_clone).await.unwrap();
    })
  };

  let reciever = {
    task::spawn(async move {
      masp_reciever.start_receiving().await.unwrap();
    })
  };

  // Wait for tasks to complete
  reciever.await?;
  video_stream.await?;
  ack_handler.await?;
  retransmitter.await?;

  Ok(())
}