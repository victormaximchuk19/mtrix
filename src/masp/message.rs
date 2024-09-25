use bytes::{BufMut, BytesMut};
use std::convert::TryFrom;

pub const MASP_MAGIC_NUMBER: [u8; 4] = [0x4D, 0x41, 0x53, 0x50]; // 'MASP'
pub const MASP_VERSION: u8 = 0x01;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketType {
  HandshakeRequest = 0x01,
  HandshakeAck = 0x02,
  HandshakeFinalAck = 0x03,
  TextData = 0x10,
  AudioData = 0x20,
  VideoData = 0x30,
  Ack = 0x40,
  RetransmissionRequest = 0x50,
  Punch = 0x60
}

impl TryFrom<u8> for PacketType {
  type Error = &'static str;

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0x01 => Ok(PacketType::HandshakeRequest),
      0x02 => Ok(PacketType::HandshakeAck),
      0x03 => Ok(PacketType::HandshakeFinalAck),
      0x10 => Ok(PacketType::TextData),
      0x20 => Ok(PacketType::AudioData),
      0x30 => Ok(PacketType::VideoData),
      0x40 => Ok(PacketType::Ack),
      0x50 => Ok(PacketType::RetransmissionRequest),
      0x60 => Ok(PacketType::Punch),
      _ => Err("Invalid packet type"),
    }
  }
}

#[derive(Clone, Debug)]
pub struct MaspPacket {
  pub version: u8,
  pub packet_type: PacketType,
  pub sequence_number: u32,
  pub payload: Vec<u8>,
}

impl MaspPacket {
  pub fn new(packet_type: PacketType, sequence_number: u32, payload: Vec<u8>) -> Self {
    Self {
      version: MASP_VERSION,
      packet_type,
      sequence_number,
      payload
    }
  }

  pub fn serialize(&self) -> Vec<u8> {
    let mut buffer = BytesMut::with_capacity(10 + self.payload.len());

    buffer.put_slice(&MASP_MAGIC_NUMBER);
    buffer.put_u8(self.version);
    buffer.put_u8(self.packet_type as u8);
    buffer.put_u32(self.sequence_number);
    buffer.put_slice(&self.payload);

    buffer.to_vec()
  }

  pub fn deserialize(buffer: &[u8]) -> Result<Self, &'static str> {
    if buffer.len() < 10 {
      return Err("Packet too short");
    }

    if &buffer[0..4] != MASP_MAGIC_NUMBER {
      return Err("Invalid magic number");
    }

    let version = buffer[4];

    if version != MASP_VERSION {
      return Err("Unsupported protocol version");
    }

    let packet_type = PacketType::try_from(buffer[5])?;

    let sequence_number = u32::from_be_bytes([
      buffer[6],
      buffer[7],
      buffer[8],
      buffer[9]
    ]);

    let payload = buffer[10..].to_vec();

    Ok(
      MaspPacket {
        version,
        packet_type,
        sequence_number,
        payload
      }
    )
  }
}
