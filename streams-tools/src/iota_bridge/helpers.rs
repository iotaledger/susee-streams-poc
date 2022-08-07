use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};
use iota_streams::core::Errors;

use crate::client::http::http_protocol_streams::MapStreamsErrors;

pub fn log_err_and_respond_500(err: anyhow::Error, fn_name: &str) -> Result<Response<Body>> {
    println!("[HttpClientProxy - {}] Error: {}", fn_name, err);

    // // Following implementation does not work because currently it is not possible to access
    // // The streams error value. Instead we expect a MessageLinkNotFoundInTangle error to
    // // make the susee POC run at all.
    // // TODO: Check how to access the streams error value and fix the implementation here
    // let streams_error = &MapStreamsErrors::get_indicator_for_uninitialized();
    // for cause in err.chain() {
    //     if let Some(streams_err) = cause.downcast_ref::<Errors>() {
    //         streams_error = streams_err.clone();
    //         break;
    //     }
    // }
    // let mut status_code = MapStreamsErrors::to_http_status_codes(&streams_error);

    let status_code = MapStreamsErrors::to_http_status_codes(&Errors::MessageLinkNotFoundInTangle(String::from("")));
    let builder = Response::builder()
        .status(status_code);
    builder.body(Default::default())
}