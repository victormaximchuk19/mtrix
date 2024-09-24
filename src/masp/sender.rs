use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use std::sync::Arc;
use std::net::SocketAddr;

use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use super::message::{MaspPacket, PacketType};

const MAX_HANDSHAKE_ATTEMPTS: u8 = 3;
const HANDSHAKE_TIMEOUT_SECONDS: u8 = 3;
const RETRANSMIT_TIMEOUT_MS: u8 = 100;

#[derive(Clone)]
pub struct MaspSender {
  socket: Arc<UdpSocket>,
  remote_addr: SocketAddr,
  sequence_number: u32,
  unacknowledged_packets: Arc<Mutex<HashMap<u32, MaspPacket>>>
}

impl MaspSender {
  pub async fn new (port: u16, remote_addr: SocketAddr) -> Result<Self, Box<dyn std::error::Error>> {
    let local_addr_str = "0.0.0.0";
    let local_addr = SocketAddr::new(local_addr_str.parse()?, port);

    let socket = UdpSocket::bind(local_addr).await?;

    Ok(
      MaspSender {
        socket: Arc::new(socket),
        remote_addr,
        sequence_number: 0,
        unacknowledged_packets: Arc::new(Mutex::new(HashMap::new()))
      }
    )
  }

  /// Sends a packet and stores it in unacknowledged_packets for retransmission if needed.
  pub async fn send_data(&mut self, packet_type: PacketType, payload: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    self.sequence_number = self.sequence_number.wrapping_add(1);

    let packet = MaspPacket::new(packet_type, self.sequence_number, payload);

    self.send_packet(&packet).await?;
    self.unacknowledged_packets.lock().await.insert(self.sequence_number, packet);

    Ok(())
  }

  pub async fn init_handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    let timeout = Duration::from_secs(HANDSHAKE_TIMEOUT_SECONDS as u64);

    for attempt in 0..MAX_HANDSHAKE_ATTEMPTS {
      println!("Sending handshake attempt: {}", attempt);

      let packet = MaspPacket::new(
        PacketType::HandshakeRequest, 0, Vec::new()
      );

      self.send_packet(&packet).await?;

      match self.receive_handshake_ack(timeout).await {
        Ok(_) => {
          println!("Handshake acknowledged");

          // Send final acknowledgment
          let ack_packet = MaspPacket::new(
              PacketType::HandshakeFinalAck,
              self.sequence_number,
              Vec::new()
          );
          
          self.send_packet(&ack_packet).await?;

          println!("Handshake completed");

          return Ok(());
        }
        Err(e) => {
          println!("Handshake attempt {} failed: {}", attempt, e);

          if attempt == (MAX_HANDSHAKE_ATTEMPTS - 1) {
            return Err("Handshake failed after 3 attempts".into());
          }
        }
      }
    }

    Ok(())
  }

  pub async fn handle_acknowledgments(&self) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 1024];

    loop {
      let (len, addr) = self.socket.recv_from(&mut buf).await?;
      
      if addr != self.remote_addr {
        continue;
      }

      let packet = MaspPacket::deserialize(&buf[..len])?;

      match packet.packet_type {
        PacketType::Ack => {
          let acked_sequence_number = u32::from_be_bytes([
            packet.payload[0], 
            packet.payload[1], 
            packet.payload[2], 
            packet.payload[3]
          ]);

          self.unacknowledged_packets.lock().await.remove(&acked_sequence_number);
        },
        // Handle other packet types if necessary
        _ => {}
      }
    }
  }

  /// Retransmits unacknowledged packets after a timeout.
  pub async fn retransmit_unacknowledged(&self) {
    let timeout = Duration::from_millis(RETRANSMIT_TIMEOUT_MS as u64);

    loop {
      sleep(timeout).await;
      let packets = self.unacknowledged_packets.lock().await.clone();
      
      for packet in packets.values() {
        println!("Retransmitting packet with sequence number {}", packet.sequence_number);
        
        let _ = self.send_packet(packet).await;
      }
    }
  }

  async fn receive_handshake_ack(&self, timeout: Duration) -> Result<(), &'static str> {
    let mut buf = [0u8; 1024];

    tokio::select! {
      result = self.socket.recv_from(&mut buf) => {
        match result {
          Ok((len, addr)) => {
            if addr != self.remote_addr {
              return Err("Received packet from unexpected address");
            }

            let packet = MaspPacket::deserialize(&buf[..len])?;

            match packet.packet_type {
              PacketType::HandshakeAck => Ok(()),
              _ => Err("Received unexpected packet type")
            }
          },
          Err(_) => Err("Failed to receive data")
        }
      }
      _ = sleep(timeout) => {
        return Err("Timeout waiting for handshake acknowledgment");
      }
    }
  }

  async fn send_packet(&self, packet: &MaspPacket) -> Result<(), Box<dyn std::error::Error>> {
    let data = packet.serialize();

    self.socket.send_to(&data, &self.remote_addr).await?;

    Ok(())
  }
}
