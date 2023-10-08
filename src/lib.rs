use reqwest::{self, StatusCode};
use serde_json::Value;
use discord_rich_presence::activity::{Timestamps, Assets, Button};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use chrono;
use thistermination::TerminationFull;
use std::error::Error;


#[derive(TerminationFull)]
pub enum SteamError {
    #[termination(exit_code(1), msg("Request failed."))]
    RequestFailed(#[from] reqwest::Error),
    #[termination(exit_code(2), msg("No game found."))]
    NoGameFound(),
    #[termination(exit_code(3), msg("Player not found."))]
    PlayerNotFound(),
    #[termination(exit_code(4), msg("Wrong api key."))]
    WrongAPIKey(),
    #[termination(exit_code(5), msg("Failed with status {0}."))]
    RequestStatusError(u16),
}

#[derive(Clone)]
pub struct Game {
    name: String,
    app_id :String,
    timestamp: i64
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.app_id == other.app_id
    }
}

pub enum State {
    ActivitySet(Game),
    ActivityNeedsToBeSet(Game),
    ActivityCleared,
    ActivityNeedsToBeCleared
}

pub async fn get_current_game(steam_id: &str, api_key: &str) -> Result<Game, SteamError> {
    let response = reqwest::get(format!("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}", api_key, steam_id)).await?;
    if response.status() == StatusCode::FORBIDDEN {
        return Err(SteamError::WrongAPIKey());
    } else if response.status() != StatusCode::OK {
        return Err(SteamError::RequestStatusError(response.status().as_u16()));
    }

    let json: Value = response.json().await?;
    if json["response"]["players"].as_array().ok_or(SteamError::PlayerNotFound())?.is_empty() {
        return Err(SteamError::PlayerNotFound());
    }
    let player_data = &json["response"]["players"][0];
    let game = player_data["gameextrainfo"].as_str().ok_or(SteamError::NoGameFound())?;
    let app_id = player_data["gameid"].as_str().ok_or(SteamError::NoGameFound())?;
    let start_time: i64 = chrono::Utc::now().timestamp();

    Ok(Game { name: game.to_string(), app_id: app_id.to_string(), timestamp: start_time })
}

pub fn set_current_game(client: &mut DiscordIpcClient, game: &Game)  -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn update_activity(client: &mut DiscordIpcClient, state: &State, steam_id: &str, steam_api_key: &str) -> Result<State, Box<dyn Error>> {
    let mut new_state = match get_current_game(steam_id, steam_api_key).await {
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
