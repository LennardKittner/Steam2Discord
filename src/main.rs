use reqwest::{self, StatusCode};
use clap::Parser;
use serde_json::Value;
use std::{time::Duration};
use std::thread;
use discord_rich_presence::activity::{Timestamps, Assets, Button};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use chrono;
use thiserror::Error;
use std::error::Error;

#[derive(Error, Debug)]
enum SteamError {
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

#[derive(Clone)]
struct Game {
    name: String,
    app_id :String,
    timestamp: i64
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.app_id == other.app_id
    }
}

enum State {
    ActivitySet(Game),
    ActivityNeedsToBeSet(Game),
    ActivityCleared,
    ActivityNeedsToBeCleared
}

fn get_current_game(steam_id: &String, api_key: &String) -> Result<Game, SteamError> {
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
    let start_time: i64 = chrono::Utc::now().timestamp();

    Ok(Game { name: game.to_string(), app_id: app_id.to_string(), timestamp: start_time })
}

fn set_current_game(client: &mut DiscordIpcClient, game: &Game)  -> Result<(), Box<dyn std::error::Error>> {
    let store_page_url = format!("https://store.steampowered.com/app/{}/", game.app_id);
    let buttons = vec![Button::new("Steam Store Page", store_page_url.as_str()), Button::new("GitHub Repository", "https://github.com/LennardKittner/Steam2Discord")];
    client.set_activity(activity::Activity::new()
        .details(&game.name)
        .timestamps(Timestamps::new()
            .start(game.timestamp))
        .assets(Assets::new()
            .large_image(game.app_id.as_str()))
        .buttons(buttons)
    )?;

    Ok(())
}

fn update_activity(client: &mut DiscordIpcClient, state: &State, args: &Cli) -> Result<State, Box<dyn Error>> {
    let mut new_state = match get_current_game(&args.steam_id, &args.steam_api_key) {
        Ok(game) => {
            match state {
                State::ActivitySet(old_game) => if *old_game != game { State::ActivityNeedsToBeSet(game) } else { State::ActivitySet(old_game.clone()) },
                State::ActivityNeedsToBeSet(old_game) => {
                    client.reconnect()?;
                    if *old_game != game { State::ActivityNeedsToBeSet(game) } else { State::ActivitySet(old_game.clone()) }
                }
                _ => State::ActivityNeedsToBeSet(game)
            }
        },
        Err(SteamError::NoGameFound()) => State::ActivityNeedsToBeCleared,
        Err(e) => return Err(Box::new(e))
    };
    new_state = match new_state {
        State::ActivityCleared => State::ActivityCleared,
        State::ActivityNeedsToBeCleared => {
            if let Err(_) = client.clear_activity() {
                State::ActivityNeedsToBeCleared
            } else {
                State::ActivityCleared
            }
        },
        State::ActivitySet(game) | State::ActivityNeedsToBeSet(game) => {
            if let Err(_) = set_current_game(client, &game) {
                State::ActivityNeedsToBeSet(game)
            } else {
                State::ActivitySet(game)
            }
        }
    };
    Ok(new_state)
}

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
        match update_activity(&mut client, &mut state, &args) {
            Ok(s) => state = s,
            Err(e) => {
                eprintln!("Error: {}", e)
            }
        }
        thread::sleep(Duration::from_secs(5));
    }
}

