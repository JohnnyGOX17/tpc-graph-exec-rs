use crate::node::NodeHandle;

/// Graph structure to manage nodes and their connections
pub struct Graph {
    handles: Vec<NodeHandle>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            handles: Vec::new(),
        }
    }

    pub fn add_handle(&mut self, handle: NodeHandle) {
        self.handles.push(handle);
    }

    pub fn wait(self) {
        for handle in self.handles {
            handle.join();
        }
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
