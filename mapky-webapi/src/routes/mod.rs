pub mod v0;

use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn routes() -> Router {
    let routes_v0 = v0::routes();

    let route_openapi = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/v0/openapi.json", v0::ApiDoc::openapi());

    let app = routes_v0.merge(route_openapi);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    app.layer(cors).layer(CompressionLayer::new())
}
