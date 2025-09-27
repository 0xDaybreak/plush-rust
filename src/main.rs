extern crate core;

use axum::{Json, Router, extract::State, routing::post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, query};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    sleep(Duration::from_secs(5)).await;

    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:a@postgres:5432/plush")
        .await
    {
        Ok(pool) => {
            println!("successfully connected to db");
            pool
        }
        Err(e) => {
            panic!("Couldn't establish DB connection {}", e)
        }
    };

    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = Router::new()
        .route("/api/buy-heavy", post(buy_heavy))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
async fn buy_heavy(State(pool): State<PgPool>, Json(reqs): Json<Vec<BuyRequest>>) -> Json<String> {
    for req in reqs.iter() {

        let plushes: Vec<Plush> =
            sqlx::query_as::<_, Plush>("SELECT id, name, price FROM plushies WHERE id = ANY($1)")
                .bind(&req.plush_ids)
                .fetch_all(&pool)
                .await
                .unwrap();

        let total: f64 = plushes.iter().map(|p| p.price).sum();

        sqlx::query(
            "INSERT INTO orders (customer_id, total_amount, order_date)
            VALUES ($1, $2, $3)",
        )
        .bind(req.customer_id)
        .bind(total)
        .bind(Utc::now())
        .execute(&pool)
        .await
        .unwrap();
    }

    Json(format!("Processed {} orders", reqs.len()))
}

#[derive(Serialize, Deserialize)]
pub struct BuyRequest {
    pub customer_id: i32,
    pub plush_ids: Vec<i64>,
}

#[derive(sqlx::FromRow, Serialize, Clone)]
pub struct Plush {
    pub id: i32,
    pub name: String,
    pub price: f64,
}

#[derive(sqlx::FromRow, Serialize, Clone)]
pub struct Order {
    pub id: i32,
    pub customer_id: i64,
    pub total_amount: f64,
    pub order_date:  DateTime<Utc>
}
