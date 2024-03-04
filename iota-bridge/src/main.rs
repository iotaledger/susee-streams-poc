use std::{
    net::SocketAddr,
};

use hyper::{
    Server,
    server::conn::AddrStream,
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

use tokio::{
    sync::oneshot,
    task::LocalSet,
};

use anyhow::Result;

use streams_tools::{
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT,
    iota_bridge::{
        LoraWanNodeDataStore,
        PendingRequestDataStore,
        BufferedMessageDataStore,
        buffered_message_loop::{
            run_buffered_message_loop,
            BufferedMessageLoopOptions,
        },
    },
    dao_helpers::DbFileBasedDaoManagerOptions,
    IotaBridge,
};

use susee_tools::{
    assert_data_dir_existence,
    get_data_folder_file_path,
    set_env_rust_log_variable_if_not_defined_by_env,
};

use cli::{
    IotaBridgeCli,
    ARG_KEYS,
    get_arg_matches,
};

mod cli;

async fn handle_request(mut client: IotaBridge<'_>, request: Request<Body>, addr: SocketAddr)
                        -> Result<Response<Body>, hyper::http::Error>
{
    let req_uri = request.uri().to_string();
    log::info!("Handling request from client address {} - URI: {}", addr, req_uri);
    let ret_val = client.handle_request(request).await;
    if ret_val.is_ok() {
        log::debug!("Sending response to address {} - URI: {}", addr, req_uri);
    } else {
        log::debug!("Sending erroneous response to address {} - URI: {}", addr, req_uri);
    }

    ret_val
}

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    // Combine it with a `LocalSet,  which means it can spawn !Send futures...
    let local = tokio::task::LocalSet::new();

    set_env_rust_log_variable_if_not_defined_by_env("info");
    env_logger::init();
    let matches_and_options = get_arg_matches();
    let cli = IotaBridgeCli::new(&matches_and_options, &ARG_KEYS) ;
    assert_data_dir_existence(&cli.data_dir).expect(
        format!("Could not create data_dir '{}'", cli.data_dir).as_str());
    let db_connection_opt = DbFileBasedDaoManagerOptions {
        file_path_and_name: get_data_folder_file_path(&cli.data_dir, "iota-bridge.sqlite3")
    };

    run_buffered_message_loop_in_background(&local, cli.node, db_connection_opt.clone());
    local.block_on(&rt, run(db_connection_opt.clone(), cli));
}

fn run_buffered_message_loop_in_background(local: &LocalSet, iota_node: &str, db_connection_opt: DbFileBasedDaoManagerOptions) {
    local.spawn_local(
        run_buffered_message_loop( BufferedMessageLoopOptions::new(
            iota_node,
            move || { BufferedMessageDataStore::new(db_connection_opt.clone()) }
        ))
    );
}

async fn run<'a>(db_connection_opt: DbFileBasedDaoManagerOptions, cli: IotaBridgeCli<'a>) {
    log::info!("Using node '{}' for tangle connection", cli.node);

    let lora_wan_node_store = LoraWanNodeDataStore::new(db_connection_opt.clone());
    let pending_request_store = PendingRequestDataStore::new(db_connection_opt.clone());
    let buffered_message_store = BufferedMessageDataStore::new(db_connection_opt);
    let client = IotaBridge::new(cli.node, lora_wan_node_store, pending_request_store, buffered_message_store).await;

    let mut addr: SocketAddr = ([127, 0, 0, 1], STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT).into();
    if cli.matches.is_present(cli.arg_keys.listener_ip_address_port) {
        let addr_str = cli.matches.value_of(cli.arg_keys.listener_ip_address_port).unwrap().trim();
        match addr_str.parse() {
            Ok(addr_from_cli) => addr = addr_from_cli,
            Err(e) => {
                log::info!("Could not parse listener_ip_address_port. Error: {}", e);
                return;
            }
        };
    }

    // Template from https://docs.rs/hyper/0.14.15/hyper/server/index.html
    // A `MakeService` that produces a `Service` to handle each connection.
    let make_service = make_service_fn(move |conn: &AddrStream| {
        // We have to clone the client to share it with each invocation of
        // `make_service`. If your data doesn't implement `Clone` consider using
        // an `std::sync::Arc`.
        let client_per_connection = client.clone();
        let addr = conn.remote_addr();

        // Create a `Service` for responding to the request.
        let service = service_fn(move |req| {
            let client_per_request = client_per_connection.clone();
            handle_request(client_per_request, req, addr)
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

    log::info!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        log::error!("server error: {}", e);
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