use bytes::Bytes;
use chrono::naive::NaiveDate;
use reqwest::blocking::Client;
use reqwest::{header, Url};
use serde::Deserialize;
use std::env::var;
use std::fs::{read, write};
use std::io::ErrorKind;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Deserialize)]
struct Config {
    api_key: Option<String>,
    image_dir: Option<PathBuf>,
}

fn load_config() -> Result<Config, String> {
    let home_dir = match var("HOME") {
        Ok(d) => d,
        Err(_) => return Ok(Default::default()),
    };

    let path: PathBuf = [&home_dir, ".apod"].iter().collect();
    let file_content = match read(&path) {
        Ok(bytes) => bytes,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => return Ok(Default::default()),
            _ => return Err(format!("Unable to read config: {}", e)),
        },
    };
    serde_json::from_slice(&file_content).map_err(|e| format!("Unable to parse config: {}", e))
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize)]
#[serde(tag = "media_type")]
enum MediaType {
    #[serde(rename = "image")]
    Image { hdurl: String },
    #[serde(rename = "video")]
    Video {},
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize)]
struct ApodData {
    copyright: Option<String>,
    date: NaiveDate,
    explanation: String,
    title: String,
    url: String,
    #[serde(flatten)]
    media: MediaType,
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

fn get_image_url(image_data: &ApodData) -> Result<Url, String> {
    match &image_data.media {
        MediaType::Image { hdurl: url } => {
            Url::parse(&url).map_err(|e| format!("Unable to parse hd image URL: {}", e))
        }
        MediaType::Video {} => Err("Unable to fetch image, media type is video".to_owned()),
    }
}

fn fetch_hd_image(client: &Client, url: &Url) -> Result<Bytes, String> {
    client
        .get(url.clone())
        .header(header::USER_AGENT, USER_AGENT)
        .send()
        .map_err(|e| format!("Error fetching image: {}", e))?
        .bytes()
        .map_err(|e| format!("Unable to read image: {}", e))
}

fn write_image(
    mut dir: PathBuf,
    apod_data: &ApodData,
    url: &Url,
    image: &[u8],
) -> Result<(), String> {
    let filename = url
        .path_segments()
        .and_then(|segments| segments.last())
        .map(|path_name| format!("{}_{}", apod_data.date, path_name))
        .unwrap_or_else(|| format!("{}", apod_data.date));
    dir.push(filename);

    write(&dir, image).map_err(|e| format!("Unable to write image data: {})", e))
}

fn main() -> Result<(), String> {
    let client = Client::new();

    let config = load_config()?;

    let api_key = match &config.api_key {
        None => {
            eprintln!("No api key found in config. Using DEMO_KEY");
            "DEMO_KEY"
        }
        Some(api_key) => api_key,
    };

    let apod_data = fetch_current_data(&client, api_key)?;

    let image_url = get_image_url(&apod_data)?;

    let hd_image = fetch_hd_image(&client, &image_url)?;

    write_image(
        config.image_dir.unwrap_or_else(|| PathBuf::from(".")),
        &apod_data,
        &image_url,
        &hd_image,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, json};

    #[test]
    fn test_media_type_image_deserialization() {
        // given
        let json = json! ({
            "copyright": "Nicolas Lefaudeux",
            "date": "2021-03-08",
            "explanation": "What created the unusual red tail[…]",
            "hdurl": "https://apod.nasa.gov/apod/image/2103/Neowise3Tails_Lefaudeux_1088.jpg",
            "media_type": "image",
            "service_version": "v1",
            "title": "Three Tails of Comet NEOWISE",
            "url": "https://apod.nasa.gov/apod/image/2103/Neowise3Tails_Lefaudeux_960.jpg"
        })
        .to_string();

        // when
        let parsed = from_str::<ApodData>(&json);

        // then
        let apod_data = parsed.expect("Expected successful parsing");
        assert_eq!(
            apod_data,
            ApodData {
                copyright: Some("Nicolas Lefaudeux".to_owned()),
                date: NaiveDate::from_ymd(2021, 3, 8),
                explanation: "What created the unusual red tail[…]".to_owned(),
                title: "Three Tails of Comet NEOWISE".to_owned(),
                url: "https://apod.nasa.gov/apod/image/2103/Neowise3Tails_Lefaudeux_960.jpg"
                    .to_owned(),
                media: MediaType::Image {
                    hdurl: "https://apod.nasa.gov/apod/image/2103/Neowise3Tails_Lefaudeux_1088.jpg"
                        .to_owned()
                }
            }
        )
    }

    #[test]
    fn test_media_type_video_deserialization() {
        // given
        let json = json! (  {
            "date": "2021-03-09",
            "explanation": "Is that a fossil?[…]",
            "media_type": "video",
            "service_version": "v1",
            "title": "Perseverance 360: Unusual Rocks and the Search for Life on Mars",
            "url": "https://mars.nasa.gov/layout/embed/image/mars-panorama/?id=25674"
        })
        .to_string();

        // when
        let parsed = from_str::<ApodData>(&json);

        // then
        let apod_data = parsed.expect("Expected successful parsing");
        assert_eq!(
            apod_data,
            ApodData {
                copyright: None,
                date: NaiveDate::from_ymd(2021, 3, 9),
                explanation: "Is that a fossil?[…]".to_owned(),
                title: "Perseverance 360: Unusual Rocks and the Search for Life on Mars".to_owned(),
                url: "https://mars.nasa.gov/layout/embed/image/mars-panorama/?id=25674".to_owned(),
                media: MediaType::Video {},
            }
        )
    }
}
