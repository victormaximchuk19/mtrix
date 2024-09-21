use crate::masp::receiver::MaspReceiver;

pub async fn run (port: u16) -> Result<(), Box<dyn std::error::Error>> {
  let mut receiver = MaspReceiver::new(port).await?;

  receiver.wait_for_handshake().await?;
  receiver.start_receiving().await?;

  Ok(())
}
