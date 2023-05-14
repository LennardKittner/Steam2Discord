use reqwest;
use clap::Parser;
use serde_json::Value;
use std::{time::Duration};
use std::thread;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use chrono;

#[derive(Parser)]
struct Cli {
    /// Your steam ID
    #[arg(long = "steamID")]
    steam_id: String,
    /// A steam web API key
    #[arg(long = "steamAPI")]
    steam_api_key: String,
    /// A discord Client ID (not required)
    #[arg(long = "discordClient", default_value_t = String::from("1104830381838057594"))]
    discord_client_id: String
}

fn get_current_game(steam_id: &String, api_key: &String) -> Result<String, reqwest::Error> {
    let response = reqwest::blocking::get(format!("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}", api_key, steam_id))?;
    let json: Value = response.json()?;
    let player_data = &json["response"]["players"][0];
    let game = player_data["gameextrainfo"].as_str().unwrap();
    println!("{game}");
    Ok(game.to_string())
}

fn set_current_game(discord_client_id: &String, game: &String) {
    let mut client = DiscordIpcClient::new(&discord_client_id).unwrap();
    client.connect().unwrap();
    let start_time = chrono::Utc::now().timestamp();

    let activity = activity::Activity::new()
            .state(game)
            .details("details");
    client.set_activity(activity);
}

fn main() {
    let args = Cli::parse();
    loop {
        match get_current_game(&args.steam_id, &args.steam_api_key) {
            Ok(game) => set_current_game(&args.discord_client_id, &game),
            Err(_) => println!("err"),
        }
        thread::sleep(Duration::from_secs(60));
    }
}

