use axum::extract::Query;
use axum::{Json, Router, extract::State, response::IntoResponse, routing::get};
use chrono::DateTime;
use chrono::Local;
use dotenvy::dotenv;
use serde::Deserialize;
use serde::Serialize;
use sqlx::QueryBuilder;
use sqlx::Row;
use sqlx::{PgPool, prelude::FromRow};
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
    pub search: String,
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

    let mut query_builder = QueryBuilder::new("SELECT * FROM posts ");

    let columns = ["id", "title", "content", "created_at"];

    if !params.search.trim().is_empty() {
        query_builder.push(" WHERE ");
        for (i, column) in columns.iter().enumerate() {
            if i > 0 {
                query_builder.push(" OR ");
            }
            if column == &"id" || column == &"created_at" {
                query_builder.push(format!(" {}::text ", column));
            } else {
                query_builder.push(format!(" {} ", column));
            }
            query_builder.push(" ILIKE ");
            query_builder.push_bind(format!("%{}%", params.search));
        }
    }

    query_builder.push(" ORDER BY ");
    query_builder.push(params.sort_by);
    query_builder.push(" ");
    query_builder.push(params.sort_order);
    query_builder.push(" LIMIT ").push_bind(params.page_size);
    query_builder.push(" OFFSET ").push_bind(offset);

    println!("{}", query_builder.sql());

    let query = query_builder.build_query_as::<Post>();
    let posts: Vec<Post> = query.fetch_all(&state.pool).await.unwrap();

    //////// Total Query //////////

    let mut total_query_builder = QueryBuilder::new("SELECT COUNT(*) FROM posts  ");

    if !params.search.trim().is_empty() {
        total_query_builder.push(" WHERE ");
        for (i, column) in columns.iter().enumerate() {
            if i > 0 {
                total_query_builder.push(" OR ");
            }
            if column == &"id" || column == &"created_at" {
                total_query_builder.push(format!(" {}::text ", column));
            } else {
                total_query_builder.push(format!(" {} ", column));
            }
            total_query_builder.push(" ILIKE ");
            total_query_builder.push_bind(format!("%{}%", params.search));
        }
    }

    let total_query = total_query_builder.build();
    let total_row = total_query.fetch_one(&state.pool).await.unwrap();
    let total: i64 = total_row.get::<i64, _>(0);

    Json(PaginatedResponse { data: posts, total })
}
