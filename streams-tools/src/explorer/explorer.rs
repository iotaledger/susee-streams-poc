use std::{
    net::SocketAddr,
};

use hyper::{
    Body,
    http::{HeaderValue, Request}
};

use anyhow::Result;

use tower_http::{
    trace::{
        TraceLayer,
        DefaultOnResponse,
        DefaultOnFailure
    },
    cors::{
        Any,
        CorsLayer,
    },
    LatencyUnit
};

use tracing::{
    Span,
    Level
};

use axum::{
    http::header::CONTENT_TYPE,
    Extension
};

use crate::{
    UserDataStore
};

use super::{
    app_state::{
        AppState,
        MessagesState,
    },
    router::router,
};

pub fn cors() -> CorsLayer {
    let swagger_url = "http://localhost:8001";

    CorsLayer::new()
        .allow_origin(swagger_url.parse::<HeaderValue>().unwrap())
        .allow_methods(Any)
        .allow_headers(vec![CONTENT_TYPE])
}

fn init_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed set_global_default");
}

pub struct ExplorerOptions {
    pub iota_node_url: String,
    pub wallet_filename: String,
    pub db_file_name: String,
    pub listener_ip_address_port: String,
    pub streams_user_serialization_password: String,
}

impl From<ExplorerOptions> for MessagesState {
    fn from(value: ExplorerOptions) -> Self {
        MessagesState{
            iota_node_url: value.iota_node_url,
            wallet_filename: value.wallet_filename,
            db_file_name: value.db_file_name,
            streams_user_serialization_password: value.streams_user_serialization_password,
        }
    }
}

pub async fn run_explorer_api_server(user_store: UserDataStore, options: ExplorerOptions) -> Result<()> {
    init_tracing();

    let addr: SocketAddr = options.listener_ip_address_port.parse()?;
    tracing::info!("listening on {}", addr);

    let app_state = AppState::new(
        options.into(),
        user_store,
    );

    let app = router()
        .layer(TraceLayer::new_for_http()
            .make_span_with(|_request: &Request<Body>| {
                tracing::debug_span!("http-request")
            })
            .on_request(|request: &Request<Body>, _span: &Span| {
                tracing::info!("started {} {}", request.method(), request.uri().path())
            })
            .on_response(
                DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(LatencyUnit::Micros))
            .on_body_chunk(())
            .on_eos(())
            .on_failure(
                DefaultOnFailure::new()
                .level(Level::INFO)
                .latency_unit(LatencyUnit::Micros))
        )
        .layer(cors())
        .layer(Extension(app_state));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("failed to start server");

    Ok(())
}