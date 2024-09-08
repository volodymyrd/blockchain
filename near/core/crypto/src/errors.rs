#[derive(Debug, Clone)]
pub enum ParseKeyError {
    #[error("unknown key type '{unknown_key_type}'")]
    UnknownKeyType { unknown_key_type: String },
    #[error("invalid key length: expected the input of {expected_length} bytes, but {received_length} was given")]
    InvalidLength {
        expected_length: usize,
        received_length: usize,
    },
    #[error("invalid key data: {error_message}")]
    InvalidData { error_message: String },
}
