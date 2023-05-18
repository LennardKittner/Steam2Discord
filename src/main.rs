use reqwest::{self, StatusCode};
use clap::Parser;
use serde_json::Value;
use std::{time::Duration};
use std::thread;
use discord_rich_presence::activity::{Timestamps, Assets, Button};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use chrono;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SteamError {
    #[error("Request failed.")]
    RequestFailed(#[from] reqwest::Error),
    #[error("No game found.")]
    NoGameFound(),
    #[error("Player not found.")]
    PlayerNotFound(),
    #[error("Wrong api key.")]
    WrongAPIKey(),
    #[error("Failed with status {0}.")]
    RequestStatusError(u16),
}

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

fn get_current_game(steam_id: &String, api_key: &String) -> Result<(String, String), SteamError> {
    let response = reqwest::blocking::get(format!("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}", api_key, steam_id))?;
    if response.status() == StatusCode::FORBIDDEN {
        return Err(SteamError::WrongAPIKey());
    } else if response.status() != StatusCode::OK {
        return Err(SteamError::RequestStatusError(response.status().as_u16()));
    }

    let json: Value = response.json()?;
    if json["response"]["players"].as_array().ok_or(SteamError::PlayerNotFound())?.is_empty() {
        return Err(SteamError::PlayerNotFound());
    }
    let player_data = &json["response"]["players"][0];
    let game = player_data["gameextrainfo"].as_str().ok_or(SteamError::NoGameFound())?;
    let app_id = player_data["gameid"].as_str().ok_or(SteamError::NoGameFound())?;

    Ok((game.to_string(), app_id.to_string()))
}

fn set_current_game(client: &mut DiscordIpcClient, game: &String, app_id: &String)  -> Result<(), Box<dyn std::error::Error>> {
    let start_time: i64 = chrono::Utc::now().timestamp();
    let store_page_url = format!("https://store.steampowered.com/app/{app_id}/");
    let buttons = vec![Button::new("Steam Store Page", store_page_url.as_str()), Button::new("GitHub Repository", "https://github.com/LennardKittner/Steam2Discord")];
    client.set_activity(activity::Activity::new()
        .details(&game)
        .timestamps(Timestamps::new()
            .start(start_time))
        .assets(Assets::new()
            .large_image(app_id.as_str()))
        .buttons(buttons)
    )?;

    Ok(())
}

//test not internet or discrod closing after init
fn main() {
    let args = Cli::parse();
    //Initial setup
    let mut client: DiscordIpcClient;
    //Wrong discord client ID => Error: Broken pipe (os error 32)
    match DiscordIpcClient::new(&args.discord_client_id) {
        Ok(discord_client) => client = discord_client,
        Err(_) => {
            eprintln!("Error: Failed to create the discord client. Check the client id.");
            std::process::exit(1);
        },
    };
    let mut last_game = ("".to_string(), "".to_string());
    loop {
        if let Err(_) = client.connect() {
            eprintln!("Error: Failed to connect to the discord client. Is discord running?");
            thread::sleep(Duration::from_secs(10));
        } else {
            break;
        }
    }

    //Run loop
    loop {
        let game = match get_current_game(&args.steam_id, &args.steam_api_key) {
            Ok(game) => game,
            Err(SteamError::NoGameFound()) => {
                if let Err(e) = client.clear_activity() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                last_game = ("".to_string(), "".to_string());
                last_game.clone()
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            },
        };
        if game != last_game {
            if let Err(e) = set_current_game(&mut client, &game.0, &game.1) {
                eprintln!("Error: {}", e);
                std::process::exit(2);
            }
        }
        thread::sleep(Duration::from_secs(10));
        last_game = game;
    }
}

