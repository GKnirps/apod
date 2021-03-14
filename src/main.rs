use bytes::Bytes;
use chrono::naive::NaiveDate;
use reqwest::blocking::Client;
use reqwest::header;
use serde::Deserialize;
use std::fs::write;
use std::path::PathBuf;

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

fn fetch_hd_image(client: &Client, image_data: &ApodData) -> Result<Bytes, String> {
    match &image_data.media {
        MediaType::Image { hdurl: url } => client
            .get(url)
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .map_err(|e| format!("Error fetching image: {}", e))?
            .bytes()
            .map_err(|e| format!("Unable to read image: {}", e)),
        MediaType::Video {} => Err("Unable to fetch image, media type is video".to_owned()),
    }
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
