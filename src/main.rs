use actix_files::NamedFile;
use actix_web::{cookie::time::parsing, get, web, App, HttpResponse, HttpServer, Responder};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rusqlite::{params, Connection};
use serde::{ser::Impossible, Serialize};
use std::{iter::StepBy, num::ParseIntError, sync::Mutex};
use tera::{Context, Tera};

async fn index() -> impl Responder {
    HttpResponse::Ok().body(include_str!("index.html"))
}
struct AppState {
    db: Mutex<Connection>,
    tera: Tera,
}

#[derive(serde::Deserialize)]
struct FormData {
    content: String,
    title: String,
}

async fn submit(content: web::Form<FormData>, data: web::Data<AppState>) -> impl Responder {
    let token: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let conn = data.db.lock().unwrap();
    conn.execute(
        " Insert into pastes (token,title,content) values ( ?,?,?);
        ",
        params![token,&content.title,&content.content.trim()],
    )
    .expect("Failed to load the data to database");

    HttpResponse::SeeOther()
        .append_header(("location", format!("/paste/{}", token)))
        .finish()
}

#[derive(Debug, Serialize)]
struct Paste_data {
    content: String,
    title: String,
    url: String,
}

async fn paste(tocken: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let token = tocken.into_inner();
    let conn = data.db.lock().unwrap();
    let paste_data = conn
        .query_row(
            " SELECT content,title from pastes where token = ?;
        ",
            params![token],
            |row| 
            {
                Ok(Paste_data{
                    content: row.get::<_,String>(0).unwrap(),
                    title : row.get::<_, String>(1).unwrap(),
                    url: format!("http://127.0.0.1:8080/paste/{}", token)
                })
                
            }
        )
        .unwrap();
    let mut content = Vec::new();
    content.push(paste_data);
    let mut context = Context::new();
    context.insert("pastes", &content);
    let render = data.tera.render("paste_data.html", &context).unwrap();
    HttpResponse::Ok().body(render)
}


#[derive(Debug, Serialize)]
struct DropdownItem {
    title: String,
    url: String,
}

async fn get_dropdown_values(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut items= Vec::new();
    let mut stmt = conn.prepare("SELECT title,token from pastes").unwrap();
    let mut rows = stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        println!("rows = {}",row.get::<_, String>(0).unwrap());
        items.push(
            DropdownItem { 
                title: row.get::<_,String>(0).unwrap(),
                url: format!("http://127.0.0.1:8080/paste/{}",row.get::<_, String>(1).unwrap())
             }
            
        )
    }
    println!("items = {:?}",items);

    HttpResponse::Ok().json(items)

}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = Connection::open("pastebin.db").expect("Failed to create data base");
    db.execute(
        "
    CREATE TABLE IF NOT EXISTS pastes (token TEXT PRIMARY_KEY, title TEXT, content TEXT);",
        params![],
    )
    .expect("Falied to create the database");
    let tera = match Tera::new("templates/*.html") {
        Ok(x) => x ,
        Err(err) => {panic!("error={}",err)},
    };
    let app_state = web::Data::new(AppState {
        db: Mutex::new(db),
        tera,
    });
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            /*
            .service(web::resource("/style.css").to(|| {
                 async { NamedFile::open("src/style.css")
                 }
                })) */
            .route("/", web::get().to(index))
            .route("/submit", web::post().to(submit))
            .route("/paste/{token}", web::get().to(paste))
            .route("/dropdown", web::get().to(get_dropdown_values))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
