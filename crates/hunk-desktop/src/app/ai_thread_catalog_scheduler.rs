use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct AiWorkspaceCatalogLoadScheduler {
    pending_roots: VecDeque<PathBuf>,
    max_concurrent_loads: usize,
    in_flight_loads: usize,
}

impl AiWorkspaceCatalogLoadScheduler {
    pub(crate) fn new(workspace_roots: Vec<PathBuf>, max_concurrent_loads: usize) -> Self {
        Self {
            pending_roots: workspace_roots.into(),
            max_concurrent_loads: max_concurrent_loads.max(1),
            in_flight_loads: 0,
        }
    }

    pub(crate) fn start_ready_loads(&mut self) -> Vec<PathBuf> {
        let mut ready_roots = Vec::new();
        while self.in_flight_loads < self.max_concurrent_loads {
            let Some(workspace_root) = self.pending_roots.pop_front() else {
                break;
            };
            self.in_flight_loads += 1;
            ready_roots.push(workspace_root);
        }
        ready_roots
    }

    pub(crate) fn finish_one_and_start_ready_loads(&mut self) -> Vec<PathBuf> {
        self.in_flight_loads = self.in_flight_loads.saturating_sub(1);
        self.start_ready_loads()
    }

    pub(crate) fn has_in_flight_loads(&self) -> bool {
        self.in_flight_loads > 0
    }
}
