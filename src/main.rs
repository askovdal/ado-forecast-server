use anyhow::{anyhow, Result};
use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app = Router::new().route("/", get(handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize)]
struct Params {
    task_id: u32,
}

#[derive(Deserialize)]
struct Task {
    company_task_id: u32,
    title: String,
}

#[derive(Serialize)]
struct ForecastLink {
    url: String,
}

async fn handler(query: Query<Params>) -> Result<impl IntoResponse, StatusCode> {
    get_task(query).await.map_err(|_| StatusCode::NOT_FOUND)
}

async fn get_task(Query(params): Query<Params>) -> Result<impl IntoResponse> {
    let api_key =
        env::var("FORECAST_API_KEY").expect("env variable FORECAST_API_KEY should be set");

    let client = reqwest::Client::new();
    let response = client
        // TODO: Add "board_name" query param that maps to a project ID in forecast
        .get("https://api.forecast.it/api/v3/projects/407834/tasks")
        .header("X-FORECAST-API-KEY", api_key)
        .send()
        .await?
        .json::<Vec<Task>>()
        .await?;

    let task = response
        .iter()
        .find(|&task| task.title.starts_with(&params.task_id.to_string()))
        .ok_or(anyhow!("No tasks with task ID {}", params.task_id))?;

    let forecast_link = ForecastLink {
        url: format!("https://app.forecast.it/T{}", task.company_task_id),
    };

    Ok((StatusCode::OK, Json(forecast_link)))
}
