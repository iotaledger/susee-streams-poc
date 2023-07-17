use streams_tools::{
    StreamsTransportSocket,
    SubscriberManagerPlainTextWallet
};

pub type ClientType = StreamsTransportSocket; // CaptureClient; //
type SubscriberManagerPlainTextWalletHttpClient = SubscriberManagerPlainTextWallet<ClientType>;


mod sensor_manager;

pub mod cli;
pub mod main;
mod command_fetcher;