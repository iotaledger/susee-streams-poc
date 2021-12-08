#![feature(once_cell)]

use anyhow::Result;

mod cli;

use streams_tools::{
    STREAMS_TOOLS_CONST_HTTP_PROXY_PORT,
    HttpClientProxy,
    ResponseCache,
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
    lazy::SyncLazy,
    net::SocketAddr,
    sync::{
        Arc,
        Mutex
    },
};
use hyper::server::conn::AddrStream;
use iota_streams::app::futures::executor::block_on;
use iota_streams::app::transport::tangle::client::Client;

type HttpClientProxyThreadSafe = Arc<Mutex<HttpClientProxy>>;

fn handle_request(client: HttpClientProxyThreadSafe, request: Request<Body>)
    -> Result<Response<Body>, hyper::http::Error>
{
    println!("[Tangle Proxy] Handling request {}", request.uri().to_string());
    let mut guard = client.lock().unwrap(); // unwrap poisoned Mutexes
    block_on(guard.handle_request(request))
}


static REQUEST_CACHE: SyncLazy<ResponseCache> = SyncLazy::new(|| {
    ResponseCache::default()
});

#[tokio::main]
async fn main() -> Result<()> {

    let arg_matches = get_arg_matches();
    let cli = TangleProxyCli::new(&arg_matches, &ARG_KEYS) ;
    println!("[Tangle Proxy] Using node '{}' for tangle connection", cli.node);

    let client: HttpClientProxyThreadSafe = Arc::new(Mutex::new(
        HttpClientProxy::new_from_url(cli.node, &REQUEST_CACHE)));

    let addr: SocketAddr = ([127, 0, 0, 1], STREAMS_TOOLS_CONST_HTTP_PROXY_PORT).into();

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
            async {
                handle_request(client_per_request, req)
            }
        });

        // Return the service to hyper.
        async move { Ok::<_, hyper::Error>(service) }
    });

    let mut client = Client::new_from_url(cli.node);

    let _join_worker_loop = REQUEST_CACHE.worker.start_loop(client, &REQUEST_CACHE);

    let server = Server::bind(&addr).serve(make_service);

    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
