use crate::cli::Versions;
use crate::stun::{send_binding_request, receive_stun_response_with_timeout, StunMessage, StunAttribute};

use tokio::net::UdpSocket;
use tokio::time::Duration;
use tokio::net::lookup_host;
use std::net::SocketAddr;

static STUN_SERVER: &str = "stun.l.google.com:19302";

// Uses STUN protocol to get the public address.
pub async fn run(port: u16, version: Versions) -> Result<SocketAddr, Box<dyn std::error::Error>> {
  // Resolve the STUN server addresses
  let mut stun_server_addrs = Vec::new();

  match lookup_host(STUN_SERVER).await {
    Ok(addrs) => stun_server_addrs.extend(addrs),
    Err(e) => eprintln!("Failed to resolve {}: {}", STUN_SERVER, e),
  }

  if stun_server_addrs.is_empty() {
    return Err("Could not resolve any STUN server addresses".into());
  }

  // set preffered version addresses first 
  stun_server_addrs.sort_by(
    |addr1, addr2| {
      match version {
        Versions::V4 => addr2.is_ipv4().cmp(&addr1.is_ipv4()),
        Versions::V6 => addr2.is_ipv6().cmp(&addr1.is_ipv6()),
      }
    }
  );
  
  // first el always an ip of desired version.
  // using `unwrap` beacause vec will always contain 0 el.  
  let stun_server_addr = stun_server_addrs.first().unwrap();

  // Bind to a local socket with the same address family
  let local_addr = match stun_server_addr {
    SocketAddr::V4(_) => SocketAddr::new("0.0.0.0".parse()?, port),
    SocketAddr::V6(_) => SocketAddr::new("::".parse()?, port)
  };

  let socket = UdpSocket::bind(local_addr).await?;

  // Construct and send the binding request
  let request = StunMessage::new();
  send_binding_request(&socket, stun_server_addr, &request).await?;

  // Receive the response with a timeout
  let (response, _addr) = receive_stun_response_with_timeout(
    &socket, 
    Duration::from_secs(5)
  ).await?;

  // Verify transaction ID
  if request.transaction_id != response.transaction_id {
    return Err("Transaction ID does not match".into());
  }

  // Extract the public address
  let attr = response.attributes.first().unwrap(); 
  
  match attr {
    StunAttribute::XorMappedAddress(addr) => Ok(*addr),
    StunAttribute::Unknown(_attr_type, _attr_value) => Err("Unhandled STUN attribute".into())
  }
}
