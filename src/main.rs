use clap::Parser;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use std::thread;
use std::time::Duration;
use steam_2_discord::State;
use tokio::runtime::Runtime;

/// A CLI application designed to improve Discord activities on Linux.
#[derive(Parser)]
struct Cli {
    /// A Discord Client ID; not required, but can be used to add custom icons
    #[arg(long = "discordClient", default_value_t = String::from("1104830381838057594"))]
    discord_client_id: String,
}

fn main() {
    let args = Cli::parse();
    //Initial setup
    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    };
    let mut client: DiscordIpcClient = DiscordIpcClient::new(&args.discord_client_id);
    let mut state = State::ActivityCleared;

    loop {
        if client.connect().is_err() {
            eprintln!("Error: Failed to connect to the discord client. Is discord running?");
            thread::sleep(Duration::from_secs(5));
        } else {
            break;
        }
    }
    println!("Set up successful.");

    //Run loop
    loop {
        match rt.block_on(steam_2_discord::update_activity(&mut client, &state)) {
            Ok(s) => state = s,
            Err(e) => {
                eprintln!("Error: {}", e)
            }
        }
        thread::sleep(Duration::from_secs(5));
    }
}
