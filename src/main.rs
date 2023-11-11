use anyhow::{anyhow, Result};
use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use dotenv::dotenv;
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, net::SocketAddr, time::Instant};

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
    project_name: String,
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

lazy_static! {
    static ref PROJECT_IDS: HashMap<String, u32> = HashMap::from([
        (String::from("Andel Energi - Adapt\\Andelenergi.dk"), 407834),
        (String::from("Andel Energi - Adapt\\Selvbetjening"), 404023),
    ]);
    static ref API_KEY: String =
        env::var("FORECAST_API_KEY").expect("env variable FORECAST_API_KEY should be set");
    static ref CLIENT: Client = Client::new();
}

async fn handler(query: Query<Params>) -> Result<impl IntoResponse, StatusCode> {
    get_task(query).await.map_err(|_| StatusCode::NOT_FOUND)
}

async fn get_task(Query(params): Query<Params>) -> Result<impl IntoResponse> {
    let project_id = PROJECT_IDS
        .get(&params.project_name)
        .ok_or(anyhow!("No project ID with name {}", params.project_name))?;

    let start_time = Instant::now();

    let response = CLIENT
        .get(format!(
            "https://api.forecast.it/api/v3/projects/{project_id}/tasks"
        ))
        .header("X-FORECAST-API-KEY", &*API_KEY)
        .send()
        .await?
        .json::<Vec<Task>>()
        .await?;

    let elapsed_time = start_time.elapsed();

    println!("Time taken to fetch data: {:?}", elapsed_time);

    let task = response
        .iter()
        .find(|&task| task.title.starts_with(&format!("{} ", params.task_id)))
        .ok_or(anyhow!("No task with task ID {}", params.task_id))?;

    let forecast_link = ForecastLink {
        url: format!("https://app.forecast.it/T{}", task.company_task_id),
    };

    Ok((StatusCode::OK, Json(forecast_link)))
}
