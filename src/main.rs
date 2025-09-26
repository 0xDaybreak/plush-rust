use axum::{
    routing::post,
    Json, Router, extract::State,
};
use sqlx::PgPool;
use std::net::SocketAddr;


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url).await?;

    let app = Router::new()
        .route("/api/buy-heavy", post(buy_heavy))
        .with_state(pool);

    let addr = SocketAddr::from(([127,0,0,1], 8080));
    println!("Listening on {}", addr);
    axum::Serve::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn buy_heavy(
    State(pool): State<PgPool>,
    Json(reqs): Json<Vec<BuyRequest>>,
) -> Json<String> {
    for req in reqs.iter() {
        // Fetch customer
        let customer_exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM customer WHERE id = $1)"
        )
            .bind(req.customer_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        if !customer_exists.0 {
            return Json(format!("Customer {} not found", req.customer_id));
        }

        // Fetch all plushes
        let plushes: Vec<Plush> = sqlx::query_as::<_, Plush>(
            "SELECT id, name, price FROM plush WHERE id = ANY($1)"
        )
            .bind(&req.plush_ids)
            .fetch_all(&pool)
            .await
            .unwrap();

        // Calculate total
        let mut total: f64 = plushes.iter().map(|p| p.price).sum();

        // Check recent orders in last 6 months
        let six_months_ago = Utc::now() - chrono::Duration::days(30*6);
        let recent_orders: Vec<Order> = sqlx::query_as::<_, Order>(
            "SELECT id, customer_id, total_amount, order_date
            FROM orders
            WHERE customer_id = $1 AND order_date > $2"
        )
            .bind(req.customer_id)
            .bind(six_months_ago)
            .fetch_all(&pool)
            .await
            .unwrap();

        if !recent_orders.is_empty() {
            total *= 0.9; // 10% discount
        }

        // Insert order
        sqlx::query(
            "INSERT INTO orders (customer_id, total_amount, order_date)
            VALUES ($1, $2, $3)"
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


use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
pub struct BuyRequest {
    pub customer_id: i64,
    pub plush_ids: Vec<i64>,
}

#[derive(sqlx::FromRow, Serialize, Clone)]
pub struct Plush {
    pub id: i64,
    pub name: String,
    pub price: f64,
}

#[derive(sqlx::FromRow, Serialize, Clone)]
pub struct Order {
    pub id: i64,
    pub customer_id: i64,
    pub total_amount: f64,
    pub order_date: DateTime<Utc>,
}
