use axum::{Json, Router, extract::State, routing::post};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
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

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
async fn buy_heavy(State(pool): State<PgPool>, Json(reqs): Json<Vec<BuyRequest>>) -> Json<String> {
    for req in reqs.iter() {
        // Fetch customer
        let customer_exists: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM customer WHERE id = $1)")
                .bind(req.customer_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        if !customer_exists.0 {
            return Json(format!("Customer {} not found", req.customer_id));
        }

        // Fetch all plushes
        let plushes: Vec<Plush> =
            sqlx::query_as::<_, Plush>("SELECT id, name, price FROM plush WHERE id = ANY($1)")
                .bind(&req.plush_ids)
                .fetch_all(&pool)
                .await
                .unwrap();

        // Calculate total
        let mut total: f64 = plushes.iter().map(|p| p.price).sum();

        // Check recent orders in last 6 months
        let six_months_ago = Utc::now() - chrono::Duration::days(30 * 6);
        let query = query(
            r#"
        SELECT id, customer_id, total_amount, order_date
                    FROM orders
                    WHERE customer_id = $1 AND order_date > $2
        "#,
        )
        .bind(req.customer_id)
        .bind(six_months_ago.to_string());
        let result = match query.fetch_all(&pool).await {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error executing query: {:?}", e);
                return Json("Error fetching".to_string());
            }
        };

        let recent_orders: Vec<Order> = result
            .into_iter()
            .map(|row| {
                let order = Order {
                    id: row.get("id"),
                    customer_id: row.get("customer_id"),
                    total_amount: row.get("total_amount"),
                    order_date: row.get("order_date"),
                };
                order
            })
            .collect();

        if !recent_orders.is_empty() {
            total *= 0.9; // 10% discount
        }

        // Insert order
        sqlx::query(
            "INSERT INTO orders (customer_id, total_amount, order_date)
            VALUES ($1, $2, $3)",
        )
        .bind(req.customer_id)
        .bind(total)
        .bind(Utc::now().to_string())
        .execute(&pool)
        .await
        .unwrap();
    }

    Json(format!("Processed {} orders", reqs.len()))
}

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
    pub order_date: String,
}
