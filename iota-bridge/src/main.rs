use anyhow::Result;

mod cli;

use streams_tools::{
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT,
    iota_bridge::{
        LoraWanNodeDataStore,
        PendingRequestDataStore,
    },
    IotaBridge,
};

use hyper::{
    Server,
    service::{
        make_service_fn,
        service_fn,
    },
    Body,
    http::{
        Request,
        Response
    }
};

use cli::{
    IotaBridgeCli,
    ARG_KEYS,
    get_arg_matches,
};

use std::{
    net::SocketAddr,
};
use std::rc::Rc;
use hyper::server::conn::AddrStream;
use tokio::sync::oneshot;
use rusqlite::Connection;

async fn handle_request(mut client: IotaBridge<'_>, request: Request<Body>)
                        -> Result<Response<Body>, hyper::http::Error>
{
    println!("-----------------------------------------------------------------\n\
    [IOTA Bridge] Handling request {}\n", request.uri().to_string());
    client.handle_request(request).await
}

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    // Combine it with a `LocalSet,  which means it can spawn !Send futures...
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, run());
}

async fn run() {
    env_logger::init();
    let matches_and_options = get_arg_matches();
    let cli = IotaBridgeCli::new(&matches_and_options, &ARG_KEYS) ;
    println!("[IOTA Bridge] Using node '{}' for tangle connection", cli.node);

    let file_path_and_name = "iota-bridge.sqlite3";
    let db_connection = Rc::new(Connection::open(file_path_and_name)
        .expect(format!("Error on open/create SQlite database file '{}'", file_path_and_name).as_str()));
    let lora_wan_node_store = LoraWanNodeDataStore::new_from_connection(db_connection.clone(), None);
    let pending_request_store = PendingRequestDataStore::new_from_connection(db_connection, None);
    let client = IotaBridge::new(cli.node, lora_wan_node_store, pending_request_store);

    let mut addr: SocketAddr = ([127, 0, 0, 1], STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT).into();
    if cli.matches.is_present(cli.arg_keys.listener_ip_address_port) {
        let addr_str = cli.matches.value_of(cli.arg_keys.listener_ip_address_port).unwrap().trim();
        match addr_str.parse() {
            Ok(addr_from_cli) => addr = addr_from_cli,
            Err(e) => {
                println!("[IOTA Bridge] Could not parse listener_ip_address_port. Error: {}", e);
                return;
            }
        };
    }

    // Template from https://docs.rs/hyper/0.14.15/hyper/server/index.html
    // A `MakeService` that produces a `Service` to handle each connection.
    let make_service = make_service_fn(move |_conn: &AddrStream| {
        // We have to clone the client to share it with each invocation of
        // `make_service`. If your data doesn't implement `Clone` consider using
        // an `std::sync::Arc`.
        let client_per_connection = client.clone();

        // Create a `Service` for responding to the request.
        let service = service_fn(move |req| {
            let client_per_request = client_per_connection.clone();
            handle_request(client_per_request, req)
        });

        // Return the service to hyper.
        async move { Ok::<_, hyper::Error>(service) }
    });

    let server = Server::bind(&addr).executor(LocalExec).serve(make_service);

    // Just shows that with_graceful_shutdown compiles with !Send,
    // !Sync HttpBody.
    let (_tx, rx) = oneshot::channel::<()>();
    let server = server.with_graceful_shutdown(async move {
        rx.await.ok();
    });

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

// Since the Server needs to spawn some background tasks, we needed
// to configure an Executor that can spawn !Send futures...
#[derive(Clone, Copy, Debug)]
struct LocalExec;

impl<F> hyper::rt::Executor<F> for LocalExec
    where
        F: std::future::Future + 'static, // not requiring `Send`
{
    fn execute(&self, fut: F) {
        // This will spawn into the currently running `LocalSet`.
        tokio::task::spawn_local(fut);
    }
}