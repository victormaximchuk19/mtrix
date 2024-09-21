mod cli;
mod cli_handlers;

mod stun;
mod masp;

mod video;

use cli::CommandHandler;

#[tokio::main]
async fn main() {
  embed_plist::embed_info_plist!("../Info.plist");

  let cli = CommandHandler::new();

  cli.run().await
}
