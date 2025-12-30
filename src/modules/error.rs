use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Config,
    Network,
    Feed,
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

impl AppError {
    pub fn config(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Config,
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Network,
            message: message.into(),
        }
    }

    pub fn feed(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Feed,
            message: message.into(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::Config => 2,
            ErrorKind::Network => 3,
            ErrorKind::Feed => 4,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}
