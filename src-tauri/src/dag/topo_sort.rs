use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use petgraph::Direction;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TopologicalSorter {
    execution_order: Vec<NodeIndex>,
    node_id_map: HashMap<Uuid, NodeIndex>,
    dirty: bool,
}

impl TopologicalSorter {
    pub fn new() -> Self {
        Self {
            execution_order: Vec::new(),
            node_id_map: HashMap::new(),
            dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn add_node(&mut self, node_id: Uuid, index: NodeIndex) {
        self.node_id_map.insert(node_id, index);
        self.dirty = true;
    }

    pub fn remove_node(&mut self, node_id: Uuid) {
        self.node_id_map.remove(&node_id);
        self.dirty = true;
    }

    pub fn get_node_index(&self, node_id: Uuid) -> Option<NodeIndex> {
        self.node_id_map.get(&node_id).copied()
    }

    pub fn get_node_id(&self, index: NodeIndex) -> Option<Uuid> {
        self.node_id_map
            .iter()
            .find(|(_, &idx)| idx == index)
            .map(|(id, _)| *id)
    }

    pub fn sort<N, E>(&mut self, graph: &DiGraph<N, E>) -> Result<Vec<NodeIndex>, String> {
        if !self.dirty && !self.execution_order.is_empty() {
            return Ok(self.execution_order.clone());
        }

        match toposort(graph, None) {
            Ok(order) => {
                self.execution_order = order.clone();
                self.dirty = false;
                Ok(order)
            }
            Err(cycle) => {
                let cycle_node = cycle.node_id();
                let cycle_path = self.find_cycle_path(graph, cycle_node);
                Err(format!(
                    "Cycle detected in graph at node {:?}. Cycle path: {:?}",
                    cycle_node, cycle_path
                ))
            }
        }
    }

    fn find_cycle_path<N, E>(
        &self,
        graph: &DiGraph<N, E>,
        start: NodeIndex,
    ) -> Vec<NodeIndex> {
        let mut visited = HashMap::new();
        let mut stack = Vec::new();
        self.dfs_cycle(graph, start, &mut visited, &mut stack);
        stack
    }

    fn dfs_cycle<N, E>(
        &self,
        graph: &DiGraph<N, E>,
        current: NodeIndex,
        visited: &mut HashMap<NodeIndex, bool>,
        stack: &mut Vec<NodeIndex>,
    ) -> bool {
        visited.insert(current, true);
        stack.push(current);

        for neighbor in graph.neighbors_directed(current, Direction::Outgoing) {
            if !visited.contains_key(&neighbor) {
                if self.dfs_cycle(graph, neighbor, visited, stack) {
                    return true;
                }
            } else if visited.get(&neighbor) == Some(&true) {
                let cycle_start = stack.iter().position(|&n| n == neighbor).unwrap_or(0);
                stack.drain(0..cycle_start);
                stack.push(neighbor);
                return true;
            }
        }

        visited.insert(current, false);
        stack.pop();
        false
    }

    pub fn get_execution_order(&self) -> &[NodeIndex] {
        &self.execution_order
    }

    pub fn clear(&mut self) {
        self.execution_order.clear();
        self.node_id_map.clear();
        self.dirty = true;
    }
}

impl Default for TopologicalSorter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::DiGraph;

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        let c = graph.add_node("C");

        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());

        let mut sorter = TopologicalSorter::new();
        let order = sorter.sort(&graph).unwrap();

        assert_eq!(order.len(), 3);
        assert_eq!(order[0], a);
        assert_eq!(order[1], b);
        assert_eq!(order[2], c);
    }

    #[test]
    fn test_topological_sort_cycle() {
        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");

        graph.add_edge(a, b, ());
        graph.add_edge(b, a, ());

        let mut sorter = TopologicalSorter::new();
        let result = sorter.sort(&graph);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cycle detected"));
    }

    #[test]
    fn test_dirty_flag() {
        let mut sorter = TopologicalSorter::new();
        assert!(sorter.is_dirty());

        let mut graph = DiGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        graph.add_edge(a, b, ());

        let _ = sorter.sort(&graph);
        assert!(!sorter.is_dirty());

        sorter.mark_dirty();
        assert!(sorter.is_dirty());
    }
}
