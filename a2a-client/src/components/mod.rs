//! Reusable web components for A2A interfaces

pub mod task_viewer;
pub mod streaming;

pub use task_viewer::{TaskView, MessageView};
pub use streaming::create_sse_stream;
