use std::ffi::NulError;

use rsmpeg::error::RsmpegError;
use thiserror::Error;

// #[non_exhaustive]
#[derive(Error, Debug, Eq, PartialEq)]
pub enum PlayerError {
    #[error("E: {0}")]
    Error(String),

    #[error("未知错误, 请联系开发人员.")]
    Unknown,

    #[error("NulError")]
    NulError(#[from] NulError),

    #[error("队列已满")]
    PacketQueueFull,

    #[error("队列为空")]
    PacketQueueEmpty,
}

pub type Result<T> = std::result::Result<T, PlayerError>;

impl From<RsmpegError> for PlayerError {
    fn from(e: RsmpegError) -> Self {
        PlayerError::Error(e.to_string())
    }
}
