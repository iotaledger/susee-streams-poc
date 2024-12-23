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
    helpers::*,
};

pub static SUSEE_CONST_SECRET_PASSWORD: &str = "SUSEE";
pub static SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC: u32 = 3;
pub static SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC: u32 = 3;