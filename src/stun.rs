use bytes::{Buf, BufMut, BytesMut};
use rand::Rng;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::net::UdpSocket;
use tokio::time::timeout;

pub const BINDING_REQUEST: u16 = 0x0001;
pub const MAGIC_COOKIE: u32 = 0x2112A442;

pub const XOR_MAPPED_ADDRESS: u16 = 0x0020;

pub struct StunMessage {
  pub message_type: u16,
  pub transaction_id: [u8; 12],
  pub attributes: Vec<StunAttribute>,
}

pub enum StunAttribute {
  XorMappedAddress(SocketAddr),
  Unknown(u16, Vec<u8>), // For attributes we don't parse
}

impl StunMessage {
  /// Creates a new STUN binding request with a random transaction ID.
  pub fn new() -> Self {
    let mut rng = rand::thread_rng();
    let transaction_id: [u8; 12] = rng.gen();
    StunMessage {
      message_type: BINDING_REQUEST,
      transaction_id,
      attributes: Vec::new(),
    }
  }

  /// Converts the STUN message into bytes for sending over UDP.
  pub fn to_bytes(&self) -> BytesMut {
    let mut buf = BytesMut::with_capacity(20);

    buf.put_u16(self.message_type);
    buf.put_u16(0); // Placeholder for message length
    buf.put_u32(MAGIC_COOKIE);
    buf.put_slice(&self.transaction_id);

    // Encode attributes (none for binding request)
    let attributes_bytes = BytesMut::new();
    let message_length = attributes_bytes.len() as u16;
    buf[2..4].copy_from_slice(&message_length.to_be_bytes());

    buf.unsplit(attributes_bytes);
    buf
  }

  /// Parses a STUN message from bytes received.
  pub fn from_bytes(mut buf: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
    // Read header
    let message_type = buf.get_u16();
    let message_length = buf.get_u16();
    let magic_cookie = buf.get_u32();

    if magic_cookie != MAGIC_COOKIE {
      return Err("Invalid magic cookie".into());
    }

    let mut transaction_id = [0u8; 12];
    buf.copy_to_slice(&mut transaction_id);

    // Read attributes
    let mut attributes = Vec::new();
    let mut attributes_buf = &buf[..message_length as usize];

    while attributes_buf.has_remaining() {
      let attr_type = attributes_buf.get_u16();
      let attr_length = attributes_buf.get_u16();
      let attr_value = attributes_buf.copy_to_bytes(attr_length as usize);

      match attr_type {
        XOR_MAPPED_ADDRESS => {
          let addr = parse_xor_mapped_address(&attr_value, &transaction_id)?;
          attributes.push(StunAttribute::XorMappedAddress(addr));
        }
        _ => {
          attributes.push(StunAttribute::Unknown(attr_type, attr_value.to_vec()));
        }
      }
    }

    Ok(StunMessage {
      message_type,
      transaction_id,
      attributes,
    })
  }
}

/// Parses the XOR-MAPPED-ADDRESS attribute to extract the public IP and port.
fn parse_xor_mapped_address(
  mut buf: &[u8],
  transaction_id: &[u8; 12]
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let _reserved = buf.get_u8();
    let family = buf.get_u8();
    let xport = buf.get_u16();
    let port = xport ^ ((MAGIC_COOKIE >> 16) as u16);
    
    let ip_addr = match family {
      0x01 => {
        // IPv4
        let xip = buf.get_u32();
        let ip = xip ^ MAGIC_COOKIE;
        IpAddr::V4(Ipv4Addr::from(ip))
      }
      0x02 => {
        let mut xip = [0u8; 16];
        buf.copy_to_slice(&mut xip);

        let mut ip = [0u8; 16];
        let magic_cookie_bytes = MAGIC_COOKIE.to_be_bytes();

        for i in 0..4 {
          ip[i] = xip[i] ^ magic_cookie_bytes[i];
        }

        for i in 4..16 {
          ip[i] = xip[i] ^ transaction_id[i - 4];
        } 

        IpAddr::V6(Ipv6Addr::from(ip))
      }
      _ => return Err("Unknown address family".into()),
    };

    Ok(SocketAddr::new(ip_addr, port))
}

/// Sends a STUN binding request to the specified STUN server.
pub async fn send_binding_request(
  socket: &UdpSocket,
  stun_server: &SocketAddr,
  message: &StunMessage,
) -> Result<(), Box<dyn std::error::Error>> {
  let buf = message.to_bytes();
  
  socket.send_to(&buf, stun_server).await?;
  
  Ok(())
}

/// Receives a STUN response with a timeout.
pub async fn receive_stun_response_with_timeout(
  socket: &UdpSocket,
  timeout_duration: std::time::Duration,
) -> Result<(StunMessage, SocketAddr), Box<dyn std::error::Error>> {
  let mut buf = [0u8; 1024];

  let (len, addr) = match timeout(timeout_duration, socket.recv_from(&mut buf)).await {
    Ok(result) => result?,
    Err(_e) => return Err("Timeout waiting for STUN response".into()),
  };
  
  let data = &buf[..len];
  let message = StunMessage::from_bytes(data)?;
  
  Ok((message, addr))
}
