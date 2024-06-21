pub mod cli;
pub mod main;
pub mod client_data_persistence_std;

mod sensor_manager;
mod command_fetcher;

use streams_tools::{
    StreamsTransportSocket,
    SubscriberManagerPlainTextWallet
};

pub type ClientType = StreamsTransportSocket; // CaptureClient; //
type SubscriberManagerPlainTextWalletHttpClient = SubscriberManagerPlainTextWallet<ClientType>;




