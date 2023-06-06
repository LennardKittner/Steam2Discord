use clap::Parser;
use std::{time::Duration};
use std::thread;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use steam_2_discord::State;
use tokio::runtime::Runtime;

/// A CLI application designed to improve Discord activities on Linux. However, it can also be used to enhance Discord activities on other platforms. 
#[derive(Parser)]
struct Cli {
    /// Your Steam ID
    #[arg(long = "steamID")]
    steam_id: String,
    /// A Steam web API key
    #[arg(long = "steamAPI")]
    steam_api_key: String,
    /// A Discord Client ID; not required, but can be used to add custom icons
    #[arg(long = "discordClient", default_value_t = String::from("1104830381838057594"))]
    discord_client_id: String
}

fn main() {
    let args = Cli::parse();
    //Initial setup
    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(error) => {
            eprintln!("Error: {}", error.to_string());
            std::process::exit(1);
        }
    };
    let mut client: DiscordIpcClient;
    //Wrong discord client ID => Error: Broken pipe (os error 32)
    match DiscordIpcClient::new(&args.discord_client_id) {
        Ok(discord_client) => client = discord_client,
        Err(_) => {
            eprintln!("Error: Failed to create the discord client. Check the client id.");
            std::process::exit(1);
        },
    };
    let mut state = State::ActivityCleared;

    loop {
        if let Err(_) = client.connect() {
            eprintln!("Error: Failed to connect to the discord client. Is discord running?");
            thread::sleep(Duration::from_secs(5));
        } else {
            break;
        }
    }
    println!("Set up successful.");

    //Run loop
    loop {
        match rt.block_on(steam_2_discord::update_activity(&mut client, &mut state, &args.steam_id, &args.steam_api_key)) {
            Ok(s) => state = s,
            Err(e) => {
                eprintln!("Error: {}", e)
            }
        }
        thread::sleep(Duration::from_secs(5));
    }
}

