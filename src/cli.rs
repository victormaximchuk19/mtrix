use crate::handlers;
use crate::masp::{sender::MaspSender, receiver::MaspReceiver, message::PacketType};

use std::net::SocketAddr;
use std::str::FromStr;
use clap::{Parser, Subcommand};
use tokio::task;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
  /// Specify the port to listen on
  #[arg(short, long, default_value = "55000")]
  pub port: u16,

  /// Specify the IP version to receive 
  #[arg(short, long, default_value = "v4", value_parser = Versions::from_str)]
  pub ipv: Versions,

  /// Subcommands
  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Clone, Debug, Copy)]
pub enum Versions {
  V4,
  V6
}

impl FromStr for Versions {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "v4" => Ok(Versions::V4),
      "v6" => Ok(Versions::V6),
      _ => Err(format!("Invalid IP version: {}", s)),
    }
  }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Outputs your IP address and port to connect to
    Whoami,

    /// Connects to a remote peer and starts video and audio chat
    Jackin {
        /// The IP address and port to connect to (format: ip:port)
        #[arg(value_parser = parse_socket_addr)]
        address: std::net::SocketAddr,
    },

    /// Makes the client go online and wait for incoming connections
    Jackwait,
}

/// Custom parser for SocketAddr to provide better error messages
fn parse_socket_addr(s: &str) -> Result<std::net::SocketAddr, String> {
  s.parse()
    .map_err(|_| format!("'{}' is not a valid IP:PORT address", s))
}

pub struct CommandHandler {
  cli: Cli
}

impl CommandHandler {
  pub fn new () -> Self {
    Self {
      cli: Cli::parse()
    }
  }
  
  /// Depending on the subcommand, perform the action
  pub async fn run (&self) {
    match &self.cli.command {
      Commands::Whoami => {
        Self::handle_whoami(&self).await;
      }
      Commands::Jackin { address } => {
        let _ = Self::handle_jackin(&self, *address).await;
      }
      Commands::Jackwait => {
        let _ = Self::handle_jackwait(&self).await;
      }
    }
  }

  /// Handles the 'whoami' command by discovering the public IP and port.
  async fn handle_whoami(&self) {
    match handlers::whoami::run(self.cli.port, self.cli.ipv).await {
      Ok(public_addr) => {
        println!("Your are {}", public_addr);
      }
      Err(e) => {
        eprintln!("Failed to get public address: {}", e);
      }
    }
  }

  /// Placeholder for the 'jackin' command functionality.
  async fn handle_jackin(&self, address: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let mut masp_sender = MaspSender::new(
      self.cli.port,
      address
    ).await?;

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

    // Send data (replace with actual data sending logic)
    masp_sender.send_data(PacketType::TextData, b"Hello, World".to_vec()).await?;

    // Wait for tasks to complete
    ack_handler.await?;
    retransmitter.await?;

    Ok(())
  }

  /// Placeholder for the 'jackwait' command functionality.
  async fn handle_jackwait(&self) -> Result<(), Box<dyn std::error::Error>> {
    let mut receiver = MaspReceiver::new(self.cli.port).await?;

    receiver.wait_for_handshake().await?;
    receiver.start_receiving().await?;

    Ok(())
  }
}
