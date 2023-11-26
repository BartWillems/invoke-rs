use serde::Deserialize;

use super::BatchId;

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct InvocationError {
    /// Offending node
    source_node_id: Option<String>,
    error_type: String,
    /// Actual error stack trace
    error: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct InvocationComplete {
    queue_id: String,
    queue_item_id: usize,
    queue_batch_id: BatchId,
    node: InvocationNode,
    result: InvocationResult,
}

impl InvocationComplete {
    pub fn id(&self) -> BatchId {
        self.queue_batch_id
    }

    pub fn still_in_progress(&self) -> bool {
        self.node.is_intermediate
    }

    pub fn image_path(&self) -> Option<String> {
        self.result
            .image
            .as_ref()
            .map(|image| image.image_name.clone())
    }
}

#[derive(Debug, Deserialize)]
struct InvocationNode {
    is_intermediate: bool,
}

#[derive(Debug, Deserialize)]
struct InvocationResult {
    image: Option<Image>,
}

#[derive(Debug, Deserialize)]
struct Image {
    image_name: String,
}
