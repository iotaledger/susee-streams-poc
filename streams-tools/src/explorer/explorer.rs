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
    UserDataStore,
    user_manager::multi_channel_management::MultiChannelManagerOptions,
    threading_helpers::run_background_worker_in_own_thread
};

use super::{
    app_state::{
        AppState,
        MessagesState,
    },
    router::router,
    sync_channels_loop::{
        SyncChannelsLoopOptions,
        SyncChannelsWorker
    }
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

#[derive(Clone)]
pub struct ExplorerOptions {
    pub iota_node: String,
    pub wallet_filename: String,
    pub db_file_name: String,
    pub listener_ip_address_port: String,
    pub streams_user_serialization_password: String,
}

impl From<ExplorerOptions> for MessagesState {
    fn from(value: ExplorerOptions) -> Self {
        MessagesState{
            iota_node_url: value.iota_node,
            wallet_filename: value.wallet_filename,
            db_file_name: value.db_file_name,
            streams_user_serialization_password: value.streams_user_serialization_password,
        }
    }
}

async fn run_sync_channels_loop_in_background(user_store: UserDataStore, options: ExplorerOptions) {
    let sync_channels_loop_options = SyncChannelsLoopOptions::new(
        user_store,
        MultiChannelManagerOptions {
            iota_node: options.iota_node,
            wallet_filename: options.wallet_filename,
            streams_user_serialization_password: options.streams_user_serialization_password,
            message_data_store_for_msg_caching: None,
            inx_collector_access_throttle_sleep_time_millisecs: Some(100),
        },
        options.db_file_name
    );

    let _join = run_background_worker_in_own_thread::<SyncChannelsWorker>(sync_channels_loop_options);
}

pub async fn run_explorer_api_server(user_store: UserDataStore, options: ExplorerOptions) -> Result<()> {
    init_tracing();

    let addr: SocketAddr = options.listener_ip_address_port.parse()?;

    let app_state = AppState::new(
        options.clone().into(),
        user_store.clone(),
    );

    run_sync_channels_loop_in_background(user_store.clone(), options.clone()).await;

    let app = router()
        .layer(TraceLayer::new_for_http()
            .make_span_with(|_request: &Request<Body>| {
                tracing::debug_span!("http-request")
            })
            .on_request(|request: &Request<Body>, _span: &Span| {
                tracing::info!("started {} {}", request.method(), request.uri())
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

    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("failed to start server");

    Ok(())
}