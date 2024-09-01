use core::panic;
use reqwest::Error;
use rocket::data::{Data, ToByteUnit};
use rocket::fs::FileServer;
use rocket::response::content::{self, RawHtml};

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Search {
    currentPage: u8,
    hasNextPage: bool,
    results: Vec<SearchElement>,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(serde::Deserialize, Default)]
struct SearchElement {
    id: String,
    title: String,
    url: String,
    image: String,
    releaseDate: String,
    subOrDub: String,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Select {
    episodes: Vec<SelectEpisode>,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct SelectEpisode {
    id: String,
    number: u8,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Episode {
    sources: Vec<Source>,
}

#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Source {
    url: String,
    quality: String,
    isM3U8: bool,
}

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    let path = std::env::current_dir().unwrap().join("public");
    rocket::build()
        .mount("/", FileServer::from(path))
        .mount("/", routes![search, select, index])
}

#[post("/index", data = "<data>")]
async fn index(data: Data<'_>) -> content::RawHtml<String> {
    let stream: String = data
        .open(2.mebibytes())
        .into_string()
        .await
        .unwrap()
        .into_inner()
        .to_string();
    let Ok(request) = serde_urlencoded::from_str::<Vec<(String, String)>>(&stream) else {
        panic!("error reading request headers")
    };

    if request[0].0.as_str() == "index" {
        let Ok(json) = searchapi(format!("watch/{}", request[0].1).as_str()).await else {
            panic!("error getting API response")
        };
        let elements: String = construct_index_html(get_url(json));

        RawHtml(elements)
    } else {
        panic!("invalid request type")
    }
}

fn get_url(json: String) -> String {
    let mut object: Episode = serde_json::from_str(json.as_str()).unwrap();

    object.sources.retain(|e| e.quality == "default");
    object.sources[0].url.clone()
}

fn construct_index_html(url: String) -> String {
    let object: String = format!(
        "<a href=\"https://haiku.leafatshredder.xyz/player/#{}\">enjoy!</a>",
        url
    );
    object
}

#[post("/select", data = "<data>")]
async fn select(data: Data<'_>) -> content::RawHtml<String> {
    let stream: String = data
        .open(2.mebibytes())
        .into_string()
        .await
        .unwrap()
        .into_inner()
        .to_string();
    let Ok(request) = serde_urlencoded::from_str::<Vec<(String, String)>>(&stream) else {
        panic!("error reading request headers")
    };

    if request[0].0.as_str() == "select" {
        let Ok(json) = searchapi(format!("info/{}", request[0].1).as_str()).await else {
            panic!("error getting API response")
        };
        let elements: String = construct_select_html(json);

        RawHtml(elements)
    } else {
        panic!("invalid request type")
    }
}

fn construct_select_html(json: String) -> String {
    let object: Select = serde_json::from_str(json.as_str()).unwrap();
    let element = object.episodes;

    let mut construct: String = String::new();
    for (i, e) in element.iter().enumerate() {
        construct.push_str(format!("<option value=\"{}\">{}</option>", e.id, i + 1).as_str())
    }
    construct
}

#[post("/search", data = "<data>")]
async fn search(data: Data<'_>) -> content::RawHtml<String> {
    let stream: String = data
        .open(2.mebibytes())
        .into_string()
        .await
        .unwrap()
        .into_inner()
        .to_string();
    let Ok(request) = serde_urlencoded::from_str::<Vec<(String, String)>>(&stream) else {
        panic!("error reading request headers")
    };

    if request[0].0.as_str() == "searchbox" {
        let Ok(json) = searchapi(request[0].1.as_str()).await else {
            panic!("error getting API response")
        };
        let elements: String = construct_search_html(filter_sub(json));

        RawHtml(elements)
    } else {
        panic!("invalid request type")
    }
}

fn filter_sub(json: String) -> Vec<SearchElement> {
    //filter dubs
    let mut object: Search = serde_json::from_str(json.as_str()).unwrap();
    let mut elements: Vec<SearchElement> = Vec::new();
    object.results.retain_mut(|e| {
        if e.subOrDub == "sub" {
            elements.push(std::mem::take(e));
            return false;
        }
        true
    });
    elements
}

fn construct_search_html(elements: Vec<SearchElement>) -> String {
    let mut construct: String = String::new();
    for (_i, e) in elements.iter().enumerate() {
        construct.push_str(format!("<option value=\"{}\">{}</option>", e.id, e.title).as_str())
    }
    construct
}

async fn searchapi(query: &str) -> Result<String, Error> {
    let res = reqwest::get(format!("http://localhost:3000/anime/gogoanime/{query}")).await?;
    let Ok(json) = res.text().await else {
        panic!("error deserializing")
    };
    Ok(json)
}

fn _typeof<T>(_: &T) {
    println!("{}", std::any::type_name::<T>());
}

fn _echo(data: &str) {
    println!("{data}")
}
