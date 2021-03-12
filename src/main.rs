use bytes::Bytes;
use chrono::naive::NaiveDate;
use reqwest::blocking::Client;
use reqwest::header;
use serde::Deserialize;
use std::fs::write;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize)]
struct ApodData {
    copyright: String,
    date: NaiveDate,
    explanation: String,
    hdurl: String,
    media_type: String,
    title: String,
    url: String,
}

const USER_AGENT: &str = "I CAN HAZ STARS?";

fn fetch_current_data(client: &Client, api_key: &str) -> Result<ApodData, String> {
    client
        .get("https://api.nasa.gov/planetary/apod")
        .header(header::ACCEPT, "application/json")
        .header(header::USER_AGENT, USER_AGENT)
        .query(&[("api_key", api_key)])
        .send()
        .map_err(|e| format!("Error fetching metadata: {}", e))?
        .json::<ApodData>()
        .map_err(|e| format!("Error parsing metadata: {}", e))
}

fn fetch_hd_image(client: &Client, image_data: &ApodData) -> Result<Bytes, String> {
    client
        .get(&image_data.hdurl)
        .header(header::USER_AGENT, USER_AGENT)
        .send()
        .map_err(|e| format!("Error fetching image: {}", e))?
        .bytes()
        .map_err(|e| format!("Unable to read image: {}", e))
}

fn write_image(mut dir: PathBuf, apod_data: &ApodData, image: &[u8]) -> Result<(), String> {
    // TODO: use better filename
    // TODO  don't just assume file extension
    dir.push(format!("{}.jpeg", apod_data.date));
    write(&dir, image).map_err(|e| format!("Unable to write image data: {})", e))
}

fn main() -> Result<(), String> {
    let client = Client::new();

    // TODO: add verbose option

    // TODO: make key configurable
    let apod_data = fetch_current_data(&client, "DEMO_KEY")?;

    let hd_image = fetch_hd_image(&client, &apod_data)?;

    // TODO: make directory configurable
    write_image(PathBuf::from("."), &apod_data, &hd_image)
}
