pub mod actions;
pub mod context;
pub mod ollama;

pub use actions::Action;
pub use context::AiContext;
pub use ollama::{send, AiResponse};
