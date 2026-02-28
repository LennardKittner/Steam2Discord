use discord_rich_presence::activity::{Assets, Button, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use reqwest::{self, StatusCode};
use serde_json::Value;
use std::error::Error;
use thistermination::TerminationFull;

use procfs::process::all_processes;
use std::ffi::OsString;

#[derive(TerminationFull)]
pub enum SteamError {
    #[termination(exit_code(1), msg("Request failed."))]
    RequestFailed(#[from] reqwest::Error),
    #[termination(exit_code(1), msg("proc fs error."))]
    ProcFSError(#[from] procfs::ProcError),
    #[termination(exit_code(2), msg("No game found."))]
    NoGameFound(),
    #[termination(exit_code(5), msg("Failed with status {0}."))]
    RequestStatusError(u16),
}

#[derive(Clone)]
pub struct Game {
    pub name: String,
    pub app_id: String,
    pub timestamp: i64,
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
    ActivityNeedsToBeCleared,
}

impl Game {
    pub async fn new_from_app_id(app_id: &str) -> Result<Game, SteamError> {
        let response = reqwest::get(format!(
            "https://store.steampowered.com/api/appdetails?appids={app_id}"
        ))
        .await?;
        if response.status() != StatusCode::OK {
            return Err(SteamError::RequestStatusError(response.status().as_u16()));
        }

        let json: Value = response.json().await?;
        let game_name = json[app_id]["data"]["name"]
            .as_str()
            .ok_or(SteamError::NoGameFound())?;
        let start_time: i64 = chrono::Utc::now().timestamp();

        Ok(Game {
            name: game_name.to_string(),
            app_id: app_id.to_string(),
            timestamp: start_time,
        })
    }
}

pub async fn get_current_game() -> Result<Game, SteamError> {
    let app_id_key = OsString::from("SteamAppId");

    for process in all_processes()?.flatten() {
        if let Ok(env) = process.environ()
            && let Some(app_id) = env.get(&app_id_key).cloned()
            && let Ok(app_id) = app_id.into_string()
        {
            let game = Game::new_from_app_id(&app_id).await?;
            return Ok(game);
        }
    }

    Err(SteamError::NoGameFound())
}

pub fn set_current_game(
    client: &mut DiscordIpcClient,
    game: &Game,
) -> Result<(), Box<dyn std::error::Error>> {
    let store_page_url = format!("https://store.steampowered.com/app/{}/", game.app_id);
    let buttons = vec![
        Button::new("Steam Store Page", store_page_url.as_str()),
        Button::new(
            "GitHub Repository",
            "https://github.com/LennardKittner/Steam2Discord",
        ),
    ];
    client.set_activity(
        activity::Activity::new()
            .details(&game.name)
            .timestamps(Timestamps::new().start(game.timestamp))
            .assets(Assets::new().large_image(game.app_id.as_str()))
            .buttons(buttons),
    )?;

    Ok(())
}

pub async fn update_activity(
    client: &mut DiscordIpcClient,
    state: &State,
) -> Result<State, Box<dyn Error>> {
    let mut new_state = match get_current_game().await {
        Ok(game) => match state {
            State::ActivitySet(old_game) => {
                if *old_game != game {
                    State::ActivityNeedsToBeSet(game)
                } else {
                    State::ActivitySet(old_game.clone())
                }
            }
            State::ActivityNeedsToBeSet(old_game) => {
                client.reconnect()?;
                if *old_game != game {
                    State::ActivityNeedsToBeSet(game)
                } else {
                    State::ActivitySet(old_game.clone())
                }
            }
            _ => State::ActivityNeedsToBeSet(game),
        },
        Err(SteamError::NoGameFound()) => State::ActivityNeedsToBeCleared,
        Err(e) => return Err(Box::new(e)),
    };
    new_state = match new_state {
        State::ActivityCleared => State::ActivityCleared,
        State::ActivityNeedsToBeCleared => {
            if client.clear_activity().is_err() {
                State::ActivityNeedsToBeCleared
            } else {
                State::ActivityCleared
            }
        }
        State::ActivitySet(game) | State::ActivityNeedsToBeSet(game) => {
            if set_current_game(client, &game).is_err() {
                State::ActivityNeedsToBeSet(game)
            } else {
                State::ActivitySet(game)
            }
        }
    };
    Ok(new_state)
}
