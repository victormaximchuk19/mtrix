mod cli;
mod commands;

mod stun;
mod masp;

mod video;

mod tests;

use cli::CommandHandler;

#[tokio::main]
async fn main() {
  embed_plist::embed_info_plist!("../Info.plist");

  let cli = CommandHandler::new();

  cli.run().await
}
