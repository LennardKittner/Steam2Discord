use reqwest;
use clap::Parser;
use serde_json::Value;

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

fn get_current_game(steam_id: &String, api_key: &String) -> Result<(), reqwest::Error> {
    let response = reqwest::blocking::get(format!("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}", api_key, steam_id))?;
    let json: Value = response.json()?;
    let player_data = &json["response"]["players"][0];
    let game = player_data["gameextrainfo"].as_str().unwrap();
    println!("{game}");
    Ok(())
}

fn main() {
    let args = Cli::parse();
    get_current_game(&args.steam_id, &args.steam_api_key).expect("steam api request failed");
}

