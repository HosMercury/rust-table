use axum::extract::Query;
use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use chrono::DateTime;
use chrono::Local;
use dotenvy::dotenv;
use serde::Deserialize;
use serde::Serialize;
use sqlx::QueryBuilder;
use sqlx::{PgPool, prelude::FromRow, query_as};
use std::env;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Error connecting to database");

    let app_state = AppState { pool };

    let cors = CorsLayer::very_permissive();

    let app = Router::new()
        .route("/", get(posts))
        .with_state(app_state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Local>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub page: i32,
    pub page_size: i32,
    pub sort_by: String,
    pub sort_order: String,
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64, // Total count of posts
}

pub async fn posts(
    State(state): State<AppState>,
    Query(params): Query<Params>,
) -> impl IntoResponse {
    let offset = params.page * params.page_size;

    let mut query_builder = QueryBuilder::new("SELECT * FROM posts");

    query_builder.push(" ORDER BY ");
    query_builder.push(params.sort_by);
    query_builder.push(" ");
    query_builder.push(params.sort_order);
    query_builder.push(" LIMIT ").push_bind(params.page_size);
    query_builder.push(" OFFSET ").push_bind(offset);

    let query = query_builder.build_query_as::<Post>();
    let posts: Vec<Post> = query.fetch_all(&state.pool).await.unwrap();

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
        .fetch_one(&state.pool)
        .await
        .unwrap();

    Json(PaginatedResponse {
        data: posts,
        total: total.0,
    })
}
