use crate::masp::message::{MaspPacket, PacketType};
use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{sync::Mutex, task};

use crate::video::ascii_frame;

const FINAL_ACK_TIMEOUT_SECONDS: u8 = 3;

#[derive(Clone)]
pub struct MaspReceiver {
  socket: Arc<UdpSocket>,
  pub remote_addr: Option<SocketAddr>,
  expected_sequence_number: u32,
  ascii_frames_buffer: Arc<Mutex<Vec<(String, u32)>>>
}

impl MaspReceiver {
  pub async fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
    let local_addr_str = "0.0.0.0";
    let local_addr = SocketAddr::new(local_addr_str.parse()?, port);

    let socket = UdpSocket::bind(local_addr).await?;

    Ok(
      MaspReceiver {
        socket: Arc::new(socket),
        remote_addr: None,
        expected_sequence_number: 0,
        ascii_frames_buffer: Arc::new(Mutex::new(Vec::<(String, u32)>::new()))
      }
    )
  }

  /// Waits for a handshake initiation from the sender.
  pub async fn wait_for_handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    loop {
      let mut buf = [0u8; 1024];
      let (len, addr) = self.socket.recv_from(&mut buf).await?;

      let packet = MaspPacket::deserialize(&buf[..len])?;

      match packet.packet_type {
        PacketType::HandshakeRequest => {
          self.remote_addr = Some(addr);

          println!("Received handshake request from {}", addr);

          // Send handshake acknowledgment
          let ack_packet = MaspPacket::new(
            PacketType::HandshakeAck, 
            packet.sequence_number, 
            Vec::new()
          );
          
          self.send_packet(&ack_packet, &addr).await?;

          // Wait for final acknowledgment
          let final_ack_result = self.receive_final_ack(
            Duration::from_secs(FINAL_ACK_TIMEOUT_SECONDS as u64)
          ).await;

          match final_ack_result {
            Ok(_) => {
              println!("Handshake completed with {}", addr);
              return Ok(());
            }
            Err(e) => {
              println!("Handshake error: {}", e);
            }
          }
        },
        _ => ()
      }
    }
  }

  async fn receive_final_ack(&self, timeout: Duration) -> Result<(), &'static str> {
    let mut buf = [0u8; 1024];

    tokio::select! {
      result = self.socket.recv_from(&mut buf) => {
        match result {
          Ok((len, addr)) => {
            if Some(addr) != self.remote_addr {
              return Err("Received packet from unexpected address");
            }

            let packet = MaspPacket::deserialize(&buf[..len])?;

            match packet.packet_type {
              PacketType::HandshakeFinalAck => Ok(()),
              _ => Err("Received unexpected packet type")
            }
          }
          Err(_) => Err("Failed to receive data"),
        }
      }
      _ = sleep(timeout) => {
        Err("Timeout waiting for final acknowledgment")
      }
    }
  }

    /// Starts receiving data packets.
  pub async fn start_receiving(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 10000];

    loop {
      let (len, addr) = self.socket.recv_from(&mut buf).await?;

      if Some(addr) != self.remote_addr {
        continue;
      }

      let packet = match MaspPacket::deserialize(&buf[..len]) {
        Ok(pkt) => pkt,
        Err(e) => {
          println!("Failed to deserialize packet: {}", e);
          continue;
        }
      };

      self.expected_sequence_number = self.expected_sequence_number.wrapping_add(1);

      match packet.packet_type {
        PacketType::TextData => {
          // Handle text data          
        }
        PacketType::AudioData => {
          // Handle audio data
        }
        PacketType::VideoData => {
          self.save_frame(packet).await?;
          self.render_frame().await;
        }
        PacketType::HandshakeRequest | PacketType::HandshakeAck | PacketType::HandshakeFinalAck => {
          // Ignore handshake packets after establishment
        }
        _ => {
          // Handle other packet types if necessary
        }
      }
    }
  }

  async fn save_frame(&mut self, packet: MaspPacket) -> Result<(), Box<dyn std::error::Error>> {
    let sequence_number = packet.sequence_number;
    self.expected_sequence_number = sequence_number;

    let decompressed_frame = ascii_frame::decompress_ascii_image(packet.payload.clone());

    let frame_data = (decompressed_frame, sequence_number);

    self.ascii_frames_buffer.lock().await.push(frame_data);
    self.send_ack(sequence_number).await?;

    Ok(())
  }

  async fn render_frame(&mut self) -> () {
    let mut locked_buf = self.ascii_frames_buffer.lock().await;
    let mut cloned_buf = locked_buf.clone(); 

    task::spawn(async move {
      if cloned_buf.len() < 24 {
        ()
      }
      
      cloned_buf.sort_by(|prv, nxt| {
        nxt.1.cmp(&prv.1)
      });

      let (frame, _) = cloned_buf.first().unwrap();
      ascii_frame::render(frame);

      cloned_buf.remove(0);
    }).await.unwrap();

    locked_buf.remove(0);
  }

  async fn send_ack(&self, sequence_number: u32 ) -> Result<(), Box<dyn std::error::Error>> {
    let ack_packet = MaspPacket::new(
      PacketType::Ack,
      0,
      sequence_number.to_be_bytes().to_vec(),
    );
    
    if let Some(addr) = self.remote_addr {
      self.send_packet(&ack_packet, &addr).await?;
    }

    Ok(())
  }

  async fn send_packet(&self, packet: &MaspPacket, addr: &SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let data = packet.serialize();
    self.socket.send_to(&data, addr).await?;
    Ok(())
  }
}
