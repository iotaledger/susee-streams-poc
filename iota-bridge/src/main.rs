use anyhow::Result;

mod cli;

use streams_tools::{
    STREAMS_TOOLS_CONST_HTTP_PROXY_PORT,
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
    TangleProxyCli,
    ARG_KEYS,
    get_arg_matches,
};

use std::{
    net::SocketAddr,
};
use hyper::server::conn::AddrStream;
use tokio::sync::oneshot;

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
    let arg_matches = get_arg_matches();
    let cli = TangleProxyCli::new(&arg_matches, &ARG_KEYS) ;
    println!("[IOTA Bridge] Using node '{}' for tangle connection", cli.node);

    let client = IotaBridge::new_from_url(cli.node);

    let mut addr: SocketAddr = ([127, 0, 0, 1], STREAMS_TOOLS_CONST_HTTP_PROXY_PORT).into();
    if cli.matches.is_present(cli.arg_keys.listener_ip_address_port) {
        let addr_str = cli.matches.value_of(cli.arg_keys.listener_ip_address_port).unwrap().trim();
        addr = addr_str.parse().unwrap();
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