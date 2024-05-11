use std::env;
use std::collections::HashMap;

use actix_web::http;
use actix_web::http::header;
use serde::Serialize;
use actix_web::{web, App, Error, HttpServer, HttpResponse};
use actix_files::Files;
// for sending multipart form data around
use actix_multipart::form::{tempfile::TempFile, text::Text, MultipartForm};

use handlebars::Handlebars;
use handlebars::DirectorySourceOptions;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

mod models;
mod schema;
use self::schema::cats::dsl::*;
use self::models::*;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Serialize)]
struct IndexTemplateData {
        project_name: String,
        cats: Vec<self::models::Cat>
}

// data from the new cat form
#[derive(MultipartForm)]
struct UploadForm {
    name: Text<String>,
    image: TempFile,
}

async fn index(hb: web::Data<Handlebars<'_>>,
               pool: web::Data<DbPool>,
               ) -> Result<HttpResponse, Error> {

   // let connection = &mut pool.get().unwrap();

    let cats_data = web::block(move || {
        cats.limit(100).load::<Cat>(&mut pool.get().unwrap())
    })
    .await
    .map_err(|_| HttpResponse::InternalServerError().finish());

    let data = IndexTemplateData {
        project_name: "Catdex".to_string(),
        cats: cats_data.unwrap().unwrap(),
    };

        
    let body = hb.render("index", &data).unwrap();

    Ok(HttpResponse::Ok().body(body))
}

async fn add(hb: web::Data<Handlebars<'_>>,) -> Result<HttpResponse, Error> {
    let body = hb.render("add", &{}).unwrap();

    Ok(HttpResponse::Ok().body(body))
}

async fn add_cat_form(pool: web::Data<DbPool>, 
                      MultipartForm(form): MultipartForm<UploadForm> )-> Result<HttpResponse, Error> {

    // so, there's no security here. TODO
    // should probably set the base folder somewhere else...
    let path = format!("./static/images/{}", form.image.file_name.unwrap());

    // get data from our form
    let _persisted_file = form.image.file.persist(&path); // {
    let cat_name = form.name.to_string();

    // not sure why we're doing this at this time, could be handy later if there
    // are more text fields?
    let text_fields: HashMap<&str, &str> = HashMap::from( [("name", cat_name.as_str())] );
    // println!("Field: {}  value: {}", "name",  text_fields.get("name").unwrap());

    let mut connection = pool.get()
        .expect("Can't get db connection from pool");
    
    let new_cat = NewCat {
        // if this is the only use of the hashmap, this is dumb
        name: text_fields.get("name").unwrap().to_string(),
        image_path: path
    };

    // right out of the diesel playbook
    let _saved_cat = web::block(move ||
               diesel::insert_into(cats)
               .values(&new_cat)
               .execute(&mut connection) 
               )
        .await
        .map_err(|_| {
            HttpResponse::InternalServerError().finish()
        }
     );



    // 3XX response, send a location redirect to the main page
    Ok(HttpResponse::SeeOther()
       .append_header((header::LOCATION, "/"))
       .finish())
}

async fn cat(
    hb: web::Data<Handlebars<'_>>,
    pool: web::Data<DbPool>,
    cat_id: web::Path<i32>,
    ) -> Result<HttpResponse, Error> {
    //TODO
    let mut connection = pool.get()
        .expect("Cannot get connection from pool");

    let cat_data = web::block(move || {
        cats.filter(id.eq(cat_id.into_inner()))
            .first::<Cat>(&mut connection)
    })
    .await
    .map_err(|_| HttpResponse::InternalServerError().finish());

    let body = hb.render("catdetail", &cat_data.unwrap().unwrap()).unwrap();

    Ok(HttpResponse::Ok().body(body))
    }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // set up the handlebar template engine
    let mut handlebars = Handlebars::new();
    println!("STarting server");

    handlebars
        .register_templates_directory("./static/", 
                                      DirectorySourceOptions {
                                            tpl_extension: ".html".to_owned(),
                                            hidden: false,
                                            temporary: false,
                                      },)
        .unwrap();

    let handlebars_ref = web::Data::new(handlebars);
   
    // set up the DB connection pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create DB connection pool");

    HttpServer::new(move || {
        App::new()
            .app_data(handlebars_ref.clone())
            .data(pool.clone())
            .service(
                Files::new("/static", "static")
                // turn this off for PROD.  BAD.
                .show_files_listing(),
                )
            .route("/", web::get().to(index))
            .route("/add", web::get().to(add))
            .route("add_cat_form", web::post().to(add_cat_form))
            .route("/cat/{id}", web::get().to(cat))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

