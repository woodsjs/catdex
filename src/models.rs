use serde::{Serialize, Deserialize};
use diesel::prelude::*;

use super::schema::cats;

// this is our schema for when the cat is populated in the DB already
#[derive(Queryable, Serialize)]
pub struct Cat {
    pub id: i32,
    pub name: String,
    pub image_path: String
}

// this is our schema for when the cat is going to be populated into the DB
// we need insertable for Diesel
// Diesel is also using table_name, it can use the struct name,
// but for whatever reason we cna't here.
#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = cats)]
pub struct NewCat {
    // id is added by the DB
    pub name: String,
    pub image_path: String,
}
