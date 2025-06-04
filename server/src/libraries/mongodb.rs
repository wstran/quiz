use mongodb::{
    bson::{Document},
    options::{IndexOptions},
    Client, Collection, Database, IndexModel,
};
use mongodb::results::{CreateIndexResult};
use tokio::sync::OnceCell;

use crate::env::MONGODB_URI;

pub static MONGODB: OnceCell<Database> = OnceCell::const_new();

pub async fn init_mongodb() {
    let client = Client::with_uri_str(MONGODB_URI.as_str())
        .await
        .expect("Failed to initialize MongoDB client");
    let db = client
        .default_database()
        .expect("No default database set");
    MONGODB.set(db).expect("MongoDB already initialized");
}

pub fn get_db() -> &'static Database {
    MONGODB.get().expect("Database not initialized")
}

pub fn get_collection(collection_name: &str) -> Collection<Document> {
    get_db().collection(collection_name)
}

pub async fn create_index(
    collection_name: &str,
    keys: Document,
    unique: bool,
    sparse: bool,
) -> mongodb::error::Result<CreateIndexResult> {
    let collection = get_collection(collection_name);
    let index_options = IndexOptions::builder().unique(unique).sparse(sparse).build();
    let index_model = IndexModel::builder()
        .keys(keys)
        .options(index_options)
        .build();

    collection.create_index(index_model).await
}