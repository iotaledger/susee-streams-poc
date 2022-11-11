pub mod cli_base;

#[cfg(feature = "std")]
mod helpers;

pub use {
    cli_base::NODE_ABOUT,
    cli_base::BaseArgKeys,
    cli_base::BASE_ARG_KEYS,
    cli_base::Cli,
    cli_base::PROJECT_CONSTANTS,
};

#[cfg(feature = "std")]
pub use {
    helpers::get_wallet_filename,
};

pub static SUSEE_CONST_SECRET_PASSWORD: &str = "SUSEE";

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
