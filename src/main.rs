use axum::{
    routing::get,
    Router,
    http::{self, StatusCode},
    Json, extract::Query,
};
use axum_macros::debug_handler;
use octocrab::{Page, models::Repository};
use serde::{Serialize, Deserialize};
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Deserialize)]
struct SearchParams {
    repo: String,
}

#[derive(Serialize)]
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
async fn search_repository(Query(params): Query<SearchParams>) -> Result<Json<RepositoryPage>, (StatusCode, String)> {
    let page = octocrab::instance()
        .search()
        .repositories(&params.repo)
        .sort("stars")
        .order("desc")
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RepositoryPage::from(page)))
}

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let cors = CorsLayer::new()
        .allow_headers([http::header::CONTENT_TYPE])
        .allow_origin(Any)
        .allow_methods(Any);

    let router = Router::new()
        .route("/search", get(search_repository))
        .layer(cors);

    tracing::info!("Starting server");
    Ok(router.into())
}
