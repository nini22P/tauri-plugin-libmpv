pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to create mpv handle")]
    Create,
    #[error("Failed to create mpv client handle")]
    ClientCreation,
    #[error("Failed to initialize mpv core: {0}")]
    Initialize(String),
    #[error("Failed to set option '{name}': {message}")]
    SetOption { name: String, message: String },
    #[error("Failed to execute command '{name}': {message}")]
    Command { name: String, message: String },
    #[error("Failed to set property '{name}': {message}")]
    SetProperty { name: String, message: String },
    #[error("Failed to get property '{name}': {message}")]
    GetProperty { name: String, message: String },
    #[error("Error processing event (id: {event_id}): {message}")]
    Event { event_id: String, message: String },
    #[error("Failed to observe property '{name}': {message}")]
    PropertyObserve { name: String, message: String },
    #[error("Invalid C-style string provided")]
    InvalidCString(#[from] std::ffi::NulError),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Failed to convert node data: {0}")]
    NodeConversion(String),
    #[error("Failed to convert property data: {0}")]
    PropertyConversion(String),
    #[error("This operation or format is not yet supported: {0}")]
    Unsupported(String),
}
