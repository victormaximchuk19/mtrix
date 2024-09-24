use std::net::SocketAddr;
use tokio::task;

use crate::masp::receiver::MaspReceiver;
use crate::masp::sender::MaspSender;
use crate::video;

pub async fn run (port: u16, address: SocketAddr) -> Result<(), Box<dyn std::error::Error>>{
  let mut masp_sender = MaspSender::new(
    port,
    address
  ).await?;
  let mut masp_reciever = MaspReceiver::new(port + 1).await?;

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