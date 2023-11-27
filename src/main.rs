use anyhow::{anyhow, Result};
use axum::{
    extract::{Query, State},
    headers::{authorization::Bearer, Authorization},
    http::{header::AUTHORIZATION, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router, TypedHeader,
};
use dotenv::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tower_http::cors::{AllowOrigin, CorsLayer};

struct AppState {
    project_ids: HashMap<String, u32>,
    client: Client,
    config: Config,
}

#[derive(Deserialize)]
struct Config {
    auth_token: String,
    forecast_api_key: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let state = Arc::new(AppState {
        project_ids: HashMap::from([
            (String::from("Andel Energi - Adapt\\Andelenergi.dk"), 407834),
            (String::from("Andel Energi - Adapt\\Selvbetjening"), 404023),
            (String::from("Andel Energi - Adapt"), 385854),
        ]),
        client: Client::new(),
        config: envy::from_env::<Config>().unwrap(),
    });

    let cors = CorsLayer::new()
        .allow_headers([AUTHORIZATION])
        .allow_origin(AllowOrigin::any());

    let app = Router::new()
        .route("/", get(handler).layer(cors))
        .route("/ping", get(|| async { StatusCode::OK }))
        .with_state(state);

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

async fn handler(
    State(state): State<Arc<AppState>>,
    TypedHeader(bearer): TypedHeader<Authorization<Bearer>>,
    Query(params): Query<Params>,
) -> Result<impl IntoResponse, StatusCode> {
    if bearer.token() != state.config.auth_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    get_task(state, params)
        .await
        // Ignore whatever the error is and return 404 Not Found
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn get_task(state: Arc<AppState>, params: Params) -> Result<impl IntoResponse> {
    let project_id = state
        .project_ids
        .get(&params.project_name)
        .ok_or(anyhow!("No project ID with name {}", params.project_name))?;

    let response = state
        .client
        .get(format!(
            "https://api.forecast.it/api/v3/projects/{project_id}/tasks"
        ))
        .header("X-FORECAST-API-KEY", &state.config.forecast_api_key)
        .send()
        .await?
        .json::<Vec<Task>>()
        .await?;

    let task = response
        .iter()
        .find(|&task| task.title.starts_with(&format!("{} ", params.task_id)))
        .ok_or(anyhow!("No task with task ID {}", params.task_id))?;

    let forecast_link = ForecastLink {
        url: format!("https://app.forecast.it/T{}", task.company_task_id),
    };

    Ok((StatusCode::OK, Json(forecast_link)))
}
