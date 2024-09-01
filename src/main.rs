use core::panic;
use reqwest::Error;
use rocket::data::{Data, ToByteUnit};
use rocket::fs::FileServer;
use rocket::http::uncased::AsUncased;
use rocket::response::content::{self, RawHtml};

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    let path = std::env::current_dir().unwrap().join("public");
    rocket::build()
        .mount("/", FileServer::from(path))
        .mount("/", routes![search, select, index, info])
}

//respond to index query with link
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
    match object.sources.len() {
        1 => object.sources[0].url.clone(),
        _ => {
            object.sources.retain(|e| e.quality == "backup");
            object.sources[0].url.clone()
        }
    }
}

fn construct_index_html(url: String) -> String {
    let object: String = format!(
        "<a href=\"https://haiku.leafatshredder.xyz/player/#{}\">enjoy!</a>",
        url
    );
    object
}

//respond to selection with episode list
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
    for e in element.iter() {
        construct.push_str(format!("<option value=\"{}\">{}</option>", e.id, e.number).as_str())
    }
    construct
}

//respond to selection with info related
#[post("/info", data = "<data>")]
async fn info(data: Data<'_>) -> content::RawHtml<String> {
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
        let elements: String = construct_info_html(json);

        RawHtml(elements)
    } else {
        panic!("invalid request type")
    }
}

fn construct_info_html(json: String) -> String {
    let object: Select = serde_json::from_str(json.as_str()).unwrap();
    let mut construct: String = String::new();
    construct.push_str(
        format!(
            r#"
        <div id=info>
            <hr />
            <img src="{}" class="img" style="float:right;"/>
            <div class="info" style="float:left">
                <h1 class="title">{}</h1>
                <div class="fields">
                    <i>episodes: {}
        "#,
            object.image, object.title, object.totalEpisodes
        )
        .as_str(),
    );
    if object.releaseDate.is_some() {
        construct.push_str(format!(r#"<br>release year: {}"#, object.releaseDate.unwrap()).as_str())
    }
    construct.push_str(
        format!(
            r#"
        <br>status: {}</i>
    </div>
    "#,
            object.status.as_uncased()
        )
        .as_str(),
    );
    if object.description.is_some() {
        construct.push_str(format!(r#"<p>{}</p>"#, object.description.unwrap()).as_str())
    }
    construct.push_str(
        r#"
        </div>
    </div>
    "#,
    );
    construct
}

//respond to search query with possible selections
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
    for e in elements.iter() {
        construct.push_str(format!("<option value=\"{}\">{}</option>", e.id, e.title).as_str())
    }
    construct
}

//common functions
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

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct Search {
    results: Vec<SearchElement>,
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize, Default)]
struct SearchElement {
    id: String,
    title: String,
    subOrDub: String,
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct Select {
    title: String,
    image: String,
    releaseDate: Option<String>,
    description: Option<String>,
    status: String,
    totalEpisodes: u8,
    episodes: Vec<SelectEpisode>,
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct SelectEpisode {
    id: String,
    number: f32,
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct Episode {
    sources: Vec<Source>,
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct Source {
    url: String,
    quality: String,
}
