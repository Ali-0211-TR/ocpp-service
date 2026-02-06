//! WebSocket server module

mod shutdown;
mod websocket;

pub use shutdown::{ShutdownCoordinator, ShutdownSignal};
pub use websocket::OcppServer;
