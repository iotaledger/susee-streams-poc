use axum::{
    routing::{get},
    Router,
};

use utoipa::{
    OpenApi,
    Modify,
    openapi::Required,
    ToSchema,
};
use utoipa_swagger_ui::SwaggerUi;

use super::{
    nodes,
    messages,
    shared::page_dto,
};

pub async fn route_info() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "routes": ["/", "/swagger-ui", "/nodes", "/messages"],
        "routes_info": {
            "/" : "this route",
            "/swagger-ui": "OpenAPI documentation",
            "/nodes": nodes::INFO,
            "/messages": messages::INFO,
        }
    }))
}

pub fn router() -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(route_info))
        .nest("/nodes", nodes::routes())
        .nest("/messages", messages::routes())
}

#[derive(OpenApi)]
#[openapi(modifiers(&SetPagingParamsToNotRequired))]
#[openapi(
    info(title="SUSEE Message Explorer", description = "Explore messages of SUSEE nodes"),
    paths(
        nodes::index,
        nodes::get,
        messages::index,
        messages::get,
    ),
    components(
        schemas(
            nodes::Node,
            messages::Message,
            page_dto::Page<nodes::Node>,
            page_dto::Page<messages::Message>,
            page_dto::PageMeta,
            DataT,
        ),
    ),
    tags(
        (name = "susee message explorer")
    )
)]
pub struct ApiDoc;

pub struct SetPagingParamsToNotRequired;

impl Modify for SetPagingParamsToNotRequired {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        for path_tuple in &mut openapi.paths.paths {
            for op_tuple in &mut path_tuple.1.operations {
                if let Some(parameters) = &mut op_tuple.1.parameters {
                    for param in parameters {
                        if param.name == "page" || param.name == "limit" {
                            param.required = Required::False;
                        }
                    }
                }
            }
        }
    }
}

#[derive(ToSchema)]
#[schema(title="JSON array")]
struct DataT {}