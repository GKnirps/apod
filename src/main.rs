use bytes::Bytes;
use chrono::naive::NaiveDate;
use percent_encoding::percent_decode_str;
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::{header, Url};
use serde::{de, Deserialize, Deserializer};
use std::env::var;
use std::fmt::Display;
use std::fs::{read, write};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

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
    let file_content = match read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return match e.kind() {
                ErrorKind::NotFound => Ok(Default::default()),
                _ => Err(format!("Unable to read config: {e}")),
            }
        }
    };
    serde_json::from_slice(&file_content).map_err(|e| format!("Unable to parse config: {e}"))
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize)]
#[serde(tag = "media_type")]
enum MediaType {
    #[serde(rename = "image")]
    Image {
        #[serde(deserialize_with = "from_str")]
        hdurl: Url,
    },
    #[serde(rename = "video")]
    Video {},
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Deserialize)]
struct ApodData {
    copyright: Option<String>,
    date: NaiveDate,
    explanation: String,
    title: String,
    #[serde(deserialize_with = "from_str")]
    url: Url,
    #[serde(flatten)]
    media: MediaType,
}

const USER_AGENT: &str = "I CAN HAZ STARS?";

fn fetch_current_data(client: &Client, api_key: &str) -> Result<ApodData, String> {
    client
        .get("https://api.nasa.gov/planetary/apod")
        .header(header::ACCEPT, "application/json")
        .query(&[("api_key", api_key)])
        .send()
        .map_err(|e| format!("Error fetching metadata: {e}"))?
        .json::<ApodData>()
        .map_err(|e| format!("Error parsing metadata: {e}"))
}

fn get_image_url(image_data: &ApodData) -> Result<&Url, String> {
    match &image_data.media {
        MediaType::Image { hdurl: url } => Ok(url),
        MediaType::Video {} => Err("Unable to fetch image, media type is video".to_owned()),
    }
}

fn fetch_hd_image(client: &Client, url: &Url) -> Result<Bytes, String> {
    client
        .get(url.clone())
        .send()
        .map_err(|e| format!("Error fetching image: {e}"))?
        .bytes()
        .map_err(|e| format!("Unable to read image: {e}"))
}

fn write_image(
    mut dir: PathBuf,
    apod_data: &ApodData,
    url: &Url,
    image: &[u8],
) -> Result<PathBuf, String> {
    dir.push(image_filename(apod_data, url));

    write(&dir, image).map_err(|e| format!("Unable to write image data: {e})"))?;

    Ok(dir)
}

fn image_filename(apod_data: &ApodData, url: &Url) -> String {
    url.path_segments()
        .and_then(|segments| segments.last())
        .and_then(|path_name| percent_decode_str(&path_name).decode_utf8().ok())
        .map(|path_name| format!("{}_{}", apod_data.date, path_name))
        .unwrap_or_else(|| format!("{}", apod_data.date))
}

fn main() -> Result<(), String> {
    let client = ClientBuilder::new()
        .user_agent(USER_AGENT)
        .tcp_keepalive(Duration::from_secs(60))
        // we only need the timeout to download images not for the json request
        // however, setting the timeout on the request does not seem to work
        .timeout(Duration::from_secs(5 * 60))
        .build()
        .map_err(|e| format!("Unable to build client: {e}"))?;

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

    let hd_image = fetch_hd_image(&client, image_url)?;

    let file_path = write_image(
        config.image_dir.unwrap_or_else(|| PathBuf::from(".")),
        &apod_data,
        image_url,
        &hd_image,
    )?;

    println!("{}", file_path.to_string_lossy());

    Ok(())
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
                date: NaiveDate::from_ymd_opt(2021, 3, 8).expect("expected valid date"),
                explanation: "What created the unusual red tail[…]".to_owned(),
                title: "Three Tails of Comet NEOWISE".to_owned(),
                url: Url::parse(
                    "https://apod.nasa.gov/apod/image/2103/Neowise3Tails_Lefaudeux_960.jpg"
                )
                .expect("Expected valid URL"),
                media: MediaType::Image {
                    hdurl: Url::parse(
                        "https://apod.nasa.gov/apod/image/2103/Neowise3Tails_Lefaudeux_1088.jpg"
                    )
                    .expect("Expected valid URL")
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
                date: NaiveDate::from_ymd_opt(2021, 3, 9).expect("expected valid date"),
                explanation: "Is that a fossil?[…]".to_owned(),
                title: "Perseverance 360: Unusual Rocks and the Search for Life on Mars".to_owned(),
                url: Url::parse("https://mars.nasa.gov/layout/embed/image/mars-panorama/?id=25674")
                    .expect("Expected valid URL"),
                media: MediaType::Video {},
            }
        )
    }

    #[test]
    fn image_filename_handles_spaces_correctly() {
        // given
        let url = Url::parse("https://apod.nasa.gov/apod/image/2404/NGC3372_ETA CARINA_LOPES.jpg")
            .expect("expected valid URL");
        let apod_data = ApodData {
            copyright: Some("Demison Lopes".to_owned()),
            date: NaiveDate::from_ymd_opt(2024, 4, 19).expect("expected valid date"),
            explanation: "A jewel of the southern sky, […]".to_owned(),
            title: "The Great Carina Nebula".to_owned(),
            url: url.clone(),
            media: MediaType::Image { hdurl: url.clone() },
        };

        // when
        let path = image_filename(&apod_data, &url);

        // then
        assert_eq!(&path, "2024-04-19_NGC3372_ETA CARINA_LOPES.jpg");
    }
}
