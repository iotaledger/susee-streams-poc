use streams_tools::{
    HttpClient,
    SubscriberManagerPlainTextWallet
};

pub type ClientType = HttpClient; // CaptureClient; //
type SubscriberManagerPlainTextWalletHttpClient = SubscriberManagerPlainTextWallet<ClientType>;


mod sensor_manager;

pub mod cli;
pub mod main;
