mod rand;

use std::path::{Path, PathBuf};

use mongodb::{
    bson::doc,
    options::{ClientOptions, UpdateOptions},
    Client,
    Database,
};
use rocket::{
    fs::{relative, NamedFile},
    get,
    http::Status,
    post,
    response::Redirect,
    routes,
    serde::{json::Json, Deserialize, Serialize},
    State,
};

use crate::rand::StringGenerator;

/// Used as a request and response type for the shorten api endpoint.
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Url {
    url: String,
}

/// The data stored in the database for each shortened url.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct UrlDocument {
    short: String,
    long:  String,
}

/// The data that is tracked for each short url.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct StatsDocument {
    short:  String,
    visits: u64,
}

#[post("/shorten", data = "<data>")]
async fn shorten(
    db: &State<Database>,
    generator: &State<StringGenerator>,
    data: Json<Url>,
) -> Result<Json<Url>, Status> {
    let urls = db.collection::<UrlDocument>("urls");

    match urls.find_one(doc! { "long": &data.url }, None).await {
        Ok(url) => {
            rocket::info!("{:?}", url);

            if let Some(url) = url {
                Ok(Json(Url { url: url.short }))
            } else {
                let generated = generator.generate_random_url_segment();
                let generated = format!("http://tier.app/r/{}", generated);

                let document = UrlDocument {
                    short: generated.clone(),
                    long:  data.url.clone(),
                };

                if let Err(e) = urls.insert_one(document, None).await {
                    // FIXME:
                    // This should be logged to something like Sentry or DataDog.
                    rocket::error!("Error: {:?}", e);

                    Err(Status::InternalServerError)
                } else {
                    Ok(Json(Url { url: generated }))
                }
            }
        },

        Err(e) => {
            // FIXME:
            // This should be logged to something like Sentry or DataDog.
            rocket::error!("Error: {:?}", e);

            // Internal Server Error
            Err(Status::InternalServerError)
        },
    }
}

#[get("/<path..>")]
async fn index(path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("public")).join(path);
    if path.is_dir() {
        path.push("index.html");
    }

    NamedFile::open(path).await.ok()
}

#[get("/r/<id>")]
async fn redirect(db: &State<Database>, id: &str) -> Redirect {
    let urls = db.collection::<UrlDocument>("urls");

    match urls.find_one(doc! { "short": id }, None).await {
        Ok(url) => {
            if let Some(url) = url {
                let stats = db.collection::<StatsDocument>("stats");

                if let Err(e) = stats
                    .update_one(
                        doc! { "short": url.short },
                        doc! {},
                        UpdateOptions::builder().upsert(true).build(),
                    )
                    .await
                {
                    // FIXME:
                    // This should be logged to something like Sentry or DataDog.
                    rocket::error!("Error: {:?}", e);
                }

                Redirect::permanent(url.long.clone())
            } else {
                Redirect::to("/index.html")
            }
        },

        Err(e) => {
            // FIXME:
            // This should be logged to something like Sentry or DataDog.
            rocket::error!("Error: {:?}", e);

            Redirect::to("/index.html")
        },
    }
}

#[rocket::main]
async fn main() -> anyhow::Result<()> {
    let options = ClientOptions::parse("mongodb://mongo:27017").await?;
    let client = Client::with_options(options)?;
    let db = client.database("urls");
    let generator = StringGenerator::new();

    rocket::build()
        .manage(db)
        .manage(generator)
        .mount("/", routes![index, redirect])
        .mount("/api/v1", routes![shorten])
        .launch()
        .await?;

    Ok(())
}
