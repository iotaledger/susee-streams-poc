use std::{
    fmt,
    str::FromStr
};

#[derive(Eq, PartialEq)]
#[derive(Clone)]
pub enum ErrorHandlingStrategy {
    AlwaysReturnErrors,
    BufferMessagesOnValidationErrors,
}

impl ErrorHandlingStrategy {
    pub const DESCRIPTION: &'static str = "Existing values are:
    always-return-errors,
        All internal errors are immediately returned to the client.
        The client is responsible to handle the error for example
        by doing a failover to another iota-bridge instance or
        by buffering the payload and later retrial.
        Use this option if there are multiple redundant iota-bridge
        instances run.
    buffer-messages-on-validation-errors
        In case the validation of a send message fails, the
        iota-bridge will buffer the message and will later retry
        to send the message via the tangle.
        This option is only suitable if only one iota-bridge
        instance is run.
    ";

    pub const ALWAYS_RETURN_ERRORS: &'static str = "always-return-errors";
    pub const BUFFER_MESSAGES_ON_VALIDATION_ERRORS: &'static str = "buffer-messages-on-validation-errors";

    pub const DEFAULT: &'static str = Self::ALWAYS_RETURN_ERRORS;

    pub fn value(&self) -> &'static str {
        match self {
            ErrorHandlingStrategy::AlwaysReturnErrors => ErrorHandlingStrategy::ALWAYS_RETURN_ERRORS,
            ErrorHandlingStrategy::BufferMessagesOnValidationErrors => ErrorHandlingStrategy::BUFFER_MESSAGES_ON_VALIDATION_ERRORS,
        }
    }
}

impl Default for ErrorHandlingStrategy {
    fn default() -> Self {
        ErrorHandlingStrategy::AlwaysReturnErrors
    }
}

impl FromStr for ErrorHandlingStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let lower_str = s.to_lowercase();
        match lower_str.as_str() {
            "always-return-errors" => Ok(ErrorHandlingStrategy::AlwaysReturnErrors),
            "buffer-messages-on-validation-errors" => Ok(ErrorHandlingStrategy::BufferMessagesOnValidationErrors),
            _ => anyhow::bail!("'{}' is not a valid ErrorHandlingStrategy value", lower_str)
        }
    }
}

impl fmt::Display for ErrorHandlingStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}