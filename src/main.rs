use std::collections::HashMap;

use axum::{
    extract::Query,
    http::{self, StatusCode},
    routing::{get, post},
    Json, Router,
};
use axum_macros::debug_handler;
use octocrab::{models::Repository, Page};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Deserialize)]
struct SearchParams {
    repo: String,
}

#[derive(Debug, Deserialize)]
struct RepoRequest {
    owner: String,
    repo: String,
}

#[derive(Debug, Serialize)]
struct RepoResponse {
    repo: Repository,
    languages: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
struct RepositoryPage {
    items: Vec<Repository>,
    #[serde(rename = "total_count")]
    total_count: usize,
    incomplete_results: bool,
}

impl From<Page<Repository>> for RepositoryPage {
    fn from(page: Page<Repository>) -> Self {
        RepositoryPage {
            items: page.items,
            total_count: page.total_count.unwrap_or_default() as usize,
            incomplete_results: page.incomplete_results.unwrap_or(false),
        }
    }
}

#[debug_handler]
async fn search_repository(
    Query(params): Query<SearchParams>,
) -> Result<Json<RepositoryPage>, (StatusCode, String)> {
    let page = octocrab::instance()
        .search()
        .repositories(&params.repo)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RepositoryPage::from(page)))
}

// TODO Simplify code
#[debug_handler]
async fn get_repository(
    Json(payload): Json<RepoRequest>,
) -> Result<Json<RepoResponse>, (StatusCode, String)> {
    let repo = octocrab::instance()
        .repos(payload.owner, payload.repo)
        .get()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // tracing::info!("{:?}", repo);
    // let languages = reqwest::get(repo.clone().languages_url.unwrap())
    //     .await
    //     .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    //     .json::<HashMap<String, u64>>()
    //     .await
    //     .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let url = repo.clone().languages_url.unwrap();
    let response = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "repos-toolbox-api")
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !response.status().is_success() {
        let error_message = format!(
            "Error fetching language data. Status code: {}",
            response.status()
        );
        return Err((StatusCode::INTERNAL_SERVER_ERROR, error_message));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response_data: HashMap<String, u64> = serde_json::from_str(&response_text)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RepoResponse {
        repo,
        languages: response_data,
    }))
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let cors = CorsLayer::new()
        .allow_headers([http::header::CONTENT_TYPE])
        .allow_origin(Any)
        .allow_methods(Any);

    let router = Router::new()
        .route("/search", get(search_repository))
        .route("/repo", post(get_repository))
        .layer(cors);

    tracing::info!("Starting server");
    Ok(router.into())
}
