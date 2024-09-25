use crate::masp::receiver::MaspReceiver;
use crate::masp::sender::MaspSender;

use std::net::SocketAddr;
use tokio::task;

use crate::video;

pub async fn run (port: u16, remote_address: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
  let local_addr_str = "0.0.0.0";
  let local_addr = SocketAddr::new(local_addr_str.parse()?, port);
  
  // SENDER PORT always equals RECIEVER PORT + 1  
  let local_sender_port = port + 1;

  let remote_address_clone = remote_address.clone();
  // shifting local SENDER socket on 1 port further
  let mut local_address_clone = local_addr.clone();
  local_address_clone.set_port(local_sender_port);
  // remote RECIEVER also to the original port
  let mut masp_sender = MaspSender::new(
    local_address_clone, 
    remote_address_clone
  ).await?;

  // UDP hole punching
  masp_sender.punch_hole(
    remote_address_clone.port(), 
    remote_address_clone.port() + 1
  ).await?;

  let mut masp_reciever = MaspReceiver::new(
    local_addr, 
    Some(remote_address)
  ).await?;

  // waiting for handshake to complete
  masp_reciever.wait_for_handshake().await?;

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

  let video_stream = task::spawn(async move {
    let sender_clone = masp_sender.clone();

    video::stream::run(sender_clone).await.unwrap();
  });

  let reciever = task::spawn(async move {
    masp_reciever.start_receiving().await.unwrap();
  });  

  // Wait for tasks to complete
  reciever.await?;
  video_stream.await?;
  ack_handler.await?;
  retransmitter.await?;

  Ok(())
}
