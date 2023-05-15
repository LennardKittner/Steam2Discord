use reqwest;
use clap::Parser;
use serde_json::Value;
use std::{time::Duration};
use std::thread;
use discord_rich_presence::activity::{Timestamps, Assets, Button};
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
    /// A discord Client ID, not required but can be used to add custom icons
    #[arg(long = "discordClient", default_value_t = String::from("1104830381838057594"))]
    discord_client_id: String
}

// TODO: error handling e.g. if no game is running
fn get_current_game(steam_id: &String, api_key: &String) -> Result<(String, i64), reqwest::Error> {
    let response = reqwest::blocking::get(format!("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}", api_key, steam_id))?;
    let json: Value = response.json()?;
    let player_data = &json["response"]["players"][0];
    let game = player_data["gameextrainfo"].as_str().unwrap();
    let app_id: i64 = player_data["gameid"].as_str().unwrap().parse::<i64>().unwrap();
    Ok((game.to_string(), app_id))
}

fn set_current_game(client: &mut DiscordIpcClient, game: &String, app_id: i64)  -> Result<(), Box<dyn std::error::Error>> {
    let start_time: i64 = chrono::Utc::now().timestamp();
    let store_page_url = format!("https://store.steampowered.com/app/{app_id}/");
    let buttons = vec![Button::new("Steam Store Page", store_page_url.as_str()), Button::new("GitHub Repository", "https://github.com/LennardKittner/Steam2Discord")];
    client.set_activity(activity::Activity::new()
        .details(&game)
        .timestamps(Timestamps::new().start(start_time))
        .assets(Assets::new()
            .large_image(app_id.to_string().as_str())
        )
        .buttons(buttons)
    )?;

    Ok(())
}

// TODO: create tool to scrape images 
fn main() {
    let args = Cli::parse();
    let mut client = DiscordIpcClient::new(&args.discord_client_id).unwrap();
    let mut last_game = ("".to_string(), -1);
    client.connect().unwrap();
    loop {
        let game = match get_current_game(&args.steam_id, &args.steam_api_key) {
            Ok(game) => game,
            Err(_) => ("".to_string(), -1),
        };
        if game != last_game {
            set_current_game(&mut client, &game.0, game.1).expect("err");
        }
        thread::sleep(Duration::from_secs(30));
        last_game = game;
    }
}

