// If you get a error at first time running this project - Install libsqlite3-dev and sqlite3
// sudo apt-get install sqlite3 libsqlite3-dev

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use rusqlite::{params, Connection};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::sync::Mutex;
use actix_files::NamedFile;
use std::fs;

// This struct holds application state( the database connection ).
// Mutex ensures that only one thread can access a shared resource
// It is used for synchronization in concurrent programming.
// Here it is used to protect access to `Connection` to prevent data races and ensure thread safety.
struct AppState {
    db: Mutex<Connection>,
}

// This async function handles the root (”/”) page of the website.
// Just returns the “index.html” page using the macro that returns the the whole file a string
async fn index() -> impl Responder {
    HttpResponse::Ok().body(include_str!("index.html"))
}

// This function is asynchronous handler for processing form submissions
// `token` is the variable that generates a random string
// `conn` locks the connection to DB using single thread only, to avoid races
// then executes the INSERT command in ‘pastes’ table with `token` and content
// Then it redirects to "/paste/token”.
async fn submit(content: web::Form<FormData>, data: web::Data<AppState>) -> impl Responder {
    let token: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    let conn = data.db.lock().unwrap();
    conn.execute(
        "INSERT INTO pastes (token, content) VALUES (?, ?)",
        params![&token, &content.content],
    )
    .expect("Failed to insert into database");

    HttpResponse::SeeOther()
        .header("Location", format!("/paste/{}", token))
        .finish()
}

// Above function handle the “/paste”,  
// `conn` locks the connection to DB.
// `content` gets the data from the pastes table using a token, gets the content.
// Returns the data in `<pre>` tag
async fn get_paste(content: web::Path<String>, data: web::Data<AppState>) -> Result<HttpResponse, actix_web::Error> {
    let conn = data.db.lock().unwrap();

    let paste_content = conn
        .query_row(
            "SELECT content FROM pastes WHERE token = ?",
            params![content.to_string()],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "Paste not found".to_string());

    // let template = fs::read_to_string("view_paste.html").unwrap_or_else(|_| "Template not found".to_string());
    // println!("template missing : {:?}", template);
    let html_page = format!(
        r#"
        <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Rustacious</title>
                <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.15/dist/tailwind.min.css" rel="stylesheet">
                <link href="https://fonts.googleapis.com/css2?family=Roboto:wght@300&display=swap" rel="stylesheet">
            </head>
            <body class="bg-gray-800 text-white" style="display: flex; flex-direction: column; justify-content: flex-start; align-items: center; height: 100vh; margin: 0;">
            <img src="https://rustacean.net/more-crabby-things/dancing-ferris.gif" alt="Rust mascot" class="logo mb-4" style="width: 16rem; height: 9rem;">
                <h2> Rusty Pastry</h2>
                    <h1  class="text-3xl mb-6">{}</h1>
            </body>
            </html>
        "#,
        paste_content
    );

    // Replace a placeholder in the template with the actual content
    // let html_page = template.replace("{content_placeholder}", &paste_content);

    // Return the HTML page as an HTTP response
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(html_page))
}


#[derive(serde::Deserialize)]
struct FormData {
    content: String,
}


// This is the main function of the project,
// 1. Tries to connect to DB
// 2. And then tries to Create the pastes table if it does not exists.
// 3. Creates the Mutex instance of AppState stucture.
// 4. Declare the HttpServer using Actix_web, with 3 routes and binds it to localhost and port 8080
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = Connection::open("pastes.db").expect("Failed to open database");
    db.execute(
        "CREATE TABLE IF NOT EXISTS pastes (token TEXT PRIMARY KEY, content TEXT)",
        params![],
    )
    .expect("Failed to create table");

    let app_state = web::Data::new(AppState {
        db: Mutex::new(db),
    });


    //Actually start the http server with 3 routes and at given port 8080
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(web::resource("/style.css").to(|| {
                async { NamedFile::open("src/style.css") }
            }))
            .route("/", web::get().to(index))
            .route("/submit", web::post().to(submit))
            .route("/paste/{token}", web::get().to(get_paste))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
