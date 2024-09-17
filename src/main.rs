mod cli;
mod stun;
mod handlers;
mod masp;

use cli::CommandHandler;

#[tokio::main]
async fn main() {
  let cli = CommandHandler::new();

  cli.run().await
}
