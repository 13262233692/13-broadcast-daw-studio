use crate::dsp::types::AudioBlock;
use crate::dsp::{Compressor, CompressorParams, Limiter, LimiterParams, ParametricEq, EqBand};
use petgraph::graph::{DiGraph, NodeIndex, EdgeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;
use uuid::Uuid;

use super::node::{
    Connection, Edge, Node, NodeProcessor, NodeType, Port, PortType, Position,
};
use super::processor::{
    CompressorNode, EqNode, GainNode, InputNode, LimiterNode, MixNode, OutputNode, Vst3Node,
};
use super::topo_sort::TopologicalSorter;

pub struct DAGProcessor {
    graph: DiGraph<Node, Edge>,
    topo_sorter: TopologicalSorter,
    node_processors: HashMap<Uuid, Box<dyn NodeProcessor>>,
    connections: Vec<Connection>,
    sample_rate: f32,
    channels: usize,
    block_size: usize,
    input_node_id: Option<Uuid>,
    output_node_id: Option<Uuid>,
}

impl DAGProcessor {
    pub fn new(sample_rate: f32, channels: usize, block_size: usize) -> Self {
        Self {
            graph: DiGraph::new(),
            topo_sorter: TopologicalSorter::new(),
            node_processors: HashMap::new(),
            connections: Vec::new(),
            sample_rate,
            channels,
            block_size,
            input_node_id: None,
            output_node_id: None,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn set_block_size(&mut self, block_size: usize) {
        self.block_size = block_size;
    }

    pub fn add_node(
        &mut self,
        node_type: NodeType,
        name: impl Into<String>,
        position: Position,
        config: serde_json::Value,
    ) -> Uuid {
        let node_id = Uuid::new_v4();

        let input_ports = match node_type {
            NodeType::Input => vec![],
            NodeType::Mix => {
                let num_inputs = config
                    .get("num_inputs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as usize;
                (0..num_inputs)
                    .map(|i| Port::new(node_id, PortType::Input, i, format!("Input {}", i + 1)))
                    .collect()
            }
            _ => vec![Port::new(node_id, PortType::Input, 0, "Input")],
        };

        let output_ports = match node_type {
            NodeType::Output => vec![],
            _ => vec![Port::new(node_id, PortType::Output, 0, "Output")],
        };

        let node = Node::with_id(
            node_id,
            node_type,
            name,
            position,
            input_ports,
            output_ports,
            config,
        );

        let node_index = self.graph.add_node(node.clone());
        self.topo_sorter.add_node(node_id, node_index);

        let processor: Box<dyn NodeProcessor> = match node_type {
            NodeType::Input => {
                self.input_node_id = Some(node_id);
                Box::new(InputNode::new(self.sample_rate, self.channels, self.block_size))
            }
            NodeType::Output => {
                self.output_node_id = Some(node_id);
                Box::new(OutputNode::new(self.sample_rate, self.channels, self.block_size))
            }
            NodeType::Mix => {
                let num_inputs = node
                    .config
                    .get("num_inputs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as usize;
                Box::new(MixNode::new(
                    self.sample_rate,
                    self.channels,
                    self.block_size,
                    num_inputs,
                ))
            }
            NodeType::Gain => {
                let gain = node
                    .config
                    .get("gain")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;
                Box::new(GainNode::with_gain(
                    self.sample_rate,
                    self.channels,
                    self.block_size,
                    gain,
                ))
            }
            NodeType::Eq => {
                let bands: Vec<EqBand> = node
                    .config
                    .get("bands")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let eq = ParametricEq::new(self.sample_rate, bands);
                Box::new(EqNode::new(eq, self.sample_rate, self.channels, self.block_size))
            }
            NodeType::Compressor => {
                let params: CompressorParams = node
                    .config
                    .get("params")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_else(|| CompressorParams::new(-20.0, 4.0, 10.0, 100.0, 0.0, 6.0));
                let compressor = Compressor::new(self.sample_rate, params);
                Box::new(CompressorNode::new(
                    compressor,
                    self.sample_rate,
                    self.channels,
                    self.block_size,
                ))
            }
            NodeType::Limiter => {
                let params: LimiterParams = node
                    .config
                    .get("params")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_else(|| LimiterParams::new(-1.0, 5.0, 100.0, 0.0));
                let limiter = Limiter::new(
                    self.sample_rate,
                    params.threshold,
                    params.attack,
                    params.release,
                    params.makeup_gain,
                );
                Box::new(LimiterNode::new(
                    limiter,
                    self.sample_rate,
                    self.channels,
                    self.block_size,
                ))
            }
            NodeType::Vst3 => Box::new(Vst3Node::new(
                self.sample_rate,
                self.channels,
                self.block_size,
            )),
        };

        self.node_processors.insert(node_id, processor);
        self.topo_sorter.mark_dirty();

        node_id
    }

    pub fn remove_node(&mut self, node_id: Uuid) -> Option<Node> {
        let node_index = self.topo_sorter.get_node_index(node_id)?;

        self.connections
            .retain(|c| c.source_node != node_id && c.target_node != node_id);

        let edges: Vec<EdgeIndex> = self
            .graph
            .edges_directed(node_index, Direction::Incoming)
            .map(|e| e.id())
            .collect();
        for edge in edges {
            self.graph.remove_edge(edge);
        }
        let edges: Vec<EdgeIndex> = self
            .graph
            .edges_directed(node_index, Direction::Outgoing)
            .map(|e| e.id())
            .collect();
        for edge in edges {
            self.graph.remove_edge(edge);
        }

        self.node_processors.remove(&node_id);
        self.topo_sorter.remove_node(node_id);

        if self.input_node_id == Some(node_id) {
            self.input_node_id = None;
        }
        if self.output_node_id == Some(node_id) {
            self.output_node_id = None;
        }

        self.topo_sorter.mark_dirty();
        self.graph.remove_node(node_index)
    }

    pub fn connect(
        &mut self,
        source_node_id: Uuid,
        source_port_id: Uuid,
        target_node_id: Uuid,
        target_port_id: Uuid,
        gain: f32,
    ) -> Result<Uuid, String> {
        let source_index = self
            .topo_sorter
            .get_node_index(source_node_id)
            .ok_or_else(|| format!("Source node {:?} not found", source_node_id))?;
        let target_index = self
            .topo_sorter
            .get_node_index(target_node_id)
            .ok_or_else(|| format!("Target node {:?} not found", target_node_id))?;

        let source_node = self
            .graph
            .node_weight(source_index)
            .ok_or_else(|| "Source node not found in graph".to_string())?;
        let target_node = self
            .graph
            .node_weight(target_index)
            .ok_or_else(|| "Target node not found in graph".to_string())?;

        source_node
            .output_ports
            .iter()
            .find(|p| p.id == source_port_id)
            .ok_or_else(|| format!("Source port {:?} not found", source_port_id))?;
        target_node
            .input_ports
            .iter()
            .find(|p| p.id == target_port_id)
            .ok_or_else(|| format!("Target port {:?} not found", target_port_id))?;

        self.graph
            .add_edge(source_index, target_index, Edge::new(gain));

        let connection = Connection::new(
            source_port_id,
            target_port_id,
            source_node_id,
            target_node_id,
            gain,
        );
        let connection_id = connection.id;
        self.connections.push(connection);

        self.topo_sorter.mark_dirty();

        Ok(connection_id)
    }

    pub fn disconnect(&mut self, connection_id: Uuid) -> Result<(), String> {
        let connection_index = self
            .connections
            .iter()
            .position(|c| c.id == connection_id)
            .ok_or_else(|| format!("Connection {:?} not found", connection_id))?;

        let connection = &self.connections[connection_index];
        let source_index = self
            .topo_sorter
            .get_node_index(connection.source_node)
            .ok_or_else(|| "Source node not found".to_string())?;
        let target_index = self
            .topo_sorter
            .get_node_index(connection.target_node)
            .ok_or_else(|| "Target node not found".to_string())?;

        let edges: Vec<EdgeIndex> = self
            .graph
            .edges_connecting(source_index, target_index)
            .map(|e| e.id())
            .collect();
        for edge in edges {
            self.graph.remove_edge(edge);
        }

        self.connections.remove(connection_index);
        self.topo_sorter.mark_dirty();

        Ok(())
    }

    pub fn get_node(&self, node_id: Uuid) -> Option<&Node> {
        let index = self.topo_sorter.get_node_index(node_id)?;
        self.graph.node_weight(index)
    }

    pub fn get_node_mut(&mut self, node_id: Uuid) -> Option<&mut Node> {
        let index = self.topo_sorter.get_node_index(node_id)?;
        self.graph.node_weight_mut(index)
    }

    pub fn get_nodes(&self) -> Vec<&Node> {
        self.graph.node_weights().collect()
    }

    pub fn get_connections(&self) -> &[Connection] {
        &self.connections
    }

    pub fn get_processor_mut<T: 'static>(&mut self, node_id: Uuid) -> Option<&mut T> {
        self.node_processors
            .get_mut(&node_id)
            .and_then(|p| p.as_any_mut().downcast_mut::<T>())
    }

    pub fn process(&mut self, input: AudioBlock) -> Result<AudioBlock, String> {
        let execution_order = self.topo_sorter.sort(&self.graph)?;

        let mut node_outputs: HashMap<NodeIndex, AudioBlock> = HashMap::new();

        for &node_index in &execution_order {
            let node = self
                .graph
                .node_weight(node_index)
                .ok_or_else(|| "Node not found in graph".to_string())?;

            if !node.enabled {
                node_outputs.insert(
                    node_index,
                    AudioBlock::new(self.channels, self.block_size, self.sample_rate),
                );
                continue;
            }

            if node.bypassed {
                let inputs: Vec<AudioBlock> = self
                    .graph
                    .edges_directed(node_index, Direction::Incoming)
                    .map(|edge| {
                        let source_index = edge.source();
                        node_outputs
                            .get(&source_index)
                            .cloned()
                            .unwrap_or_else(|| {
                                AudioBlock::new(self.channels, self.block_size, self.sample_rate)
                            })
                    })
                    .collect();

                let output = if !inputs.is_empty() {
                    inputs[0].clone()
                } else {
                    AudioBlock::new(self.channels, self.block_size, self.sample_rate)
                };
                node_outputs.insert(node_index, output);
                continue;
            }

            let mut inputs: Vec<AudioBlock> = Vec::new();

            if node.node_type == NodeType::Input {
                inputs.push(input.clone());
            } else {
                let incoming_edges: Vec<_> = self
                    .graph
                    .edges_directed(node_index, Direction::Incoming)
                    .collect();

                for edge in incoming_edges {
                    let source_index = edge.source();
                    let edge_weight = edge.weight();
                    if let Some(source_output) = node_outputs.get(&source_index) {
                        let mut processed_input = source_output.clone();

                        if (edge_weight.gain - 1.0).abs() > f32::EPSILON {
                            for ch in 0..processed_input.channels {
                                for frame in 0..processed_input.block_size {
                                    processed_input.samples[ch][frame] *= edge_weight.gain;
                                }
                            }
                        }

                        inputs.push(processed_input);
                    } else {
                        inputs.push(AudioBlock::new(
                            self.channels,
                            self.block_size,
                            self.sample_rate,
                        ));
                    }
                }
            }

            let processor = self
                .node_processors
                .get_mut(&node.id)
                .ok_or_else(|| format!("Processor for node {:?} not found", node.id))?;

            let output = processor.process(inputs);
            node_outputs.insert(node_index, output);
        }

        if let Some(output_node_id) = self.output_node_id {
            let output_index = self
                .topo_sorter
                .get_node_index(output_node_id)
                .ok_or_else(|| "Output node not found".to_string())?;
            Ok(node_outputs
                .get(&output_index)
                .cloned()
                .unwrap_or_else(|| AudioBlock::new(self.channels, self.block_size, self.sample_rate)))
        } else {
            Err("No output node found in graph".to_string())
        }
    }

    pub fn validate(&mut self) -> Result<(), String> {
        self.topo_sorter.sort(&self.graph)?;

        if self.input_node_id.is_none() {
            return Err("No input node found".to_string());
        }
        if self.output_node_id.is_none() {
            return Err("No output node found".to_string());
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.graph.clear();
        self.topo_sorter.clear();
        self.node_processors.clear();
        self.connections.clear();
        self.input_node_id = None;
        self.output_node_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_signal_chain() {
        let mut dag = DAGProcessor::new(48000.0, 2, 256);

        let input_id = dag.add_node(
            NodeType::Input,
            "Input",
            Position { x: 0.0, y: 0.0 },
            serde_json::json!({}),
        );

        let gain_id = dag.add_node(
            NodeType::Gain,
            "Gain",
            Position { x: 100.0, y: 0.0 },
            serde_json::json!({ "gain": 0.5 }),
        );

        let output_id = dag.add_node(
            NodeType::Output,
            "Output",
            Position { x: 200.0, y: 0.0 },
            serde_json::json!({}),
        );

        let input_node = dag.get_node(input_id).unwrap();
        let gain_node = dag.get_node(gain_id).unwrap();
        let output_node = dag.get_node(output_id).unwrap();

        dag.connect(
            input_id,
            input_node.output_ports[0].id,
            gain_id,
            gain_node.input_ports[0].id,
            1.0,
        )
        .unwrap();

        dag.connect(
            gain_id,
            gain_node.output_ports[0].id,
            output_id,
            output_node.input_ports[0].id,
            1.0,
        )
        .unwrap();

        let mut input = AudioBlock::new(2, 256, 48000.0);
        for ch in 0..2 {
            for frame in 0..256 {
                input.samples[ch][frame] = 1.0;
            }
        }

        let output = dag.process(input).unwrap();

        for ch in 0..2 {
            for frame in 0..256 {
                assert!((output.samples[ch][frame] - 0.5).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn test_mix_node() {
        let mut dag = DAGProcessor::new(48000.0, 2, 256);

        let input1_id = dag.add_node(
            NodeType::Input,
            "Input 1",
            Position { x: 0.0, y: 0.0 },
            serde_json::json!({}),
        );

        let input2_id = dag.add_node(
            NodeType::Input,
            "Input 2",
            Position { x: 0.0, y: 100.0 },
            serde_json::json!({}),
        );

        let mix_id = dag.add_node(
            NodeType::Mix,
            "Mixer",
            Position { x: 100.0, y: 50.0 },
            serde_json::json!({ "num_inputs": 2 }),
        );

        let output_id = dag.add_node(
            NodeType::Output,
            "Output",
            Position { x: 200.0, y: 50.0 },
            serde_json::json!({}),
        );

        let input1_node = dag.get_node(input1_id).unwrap();
        let input2_node = dag.get_node(input2_id).unwrap();
        let mix_node = dag.get_node(mix_id).unwrap();
        let output_node = dag.get_node(output_id).unwrap();

        dag.connect(
            input1_id,
            input1_node.output_ports[0].id,
            mix_id,
            mix_node.input_ports[0].id,
            1.0,
        )
        .unwrap();

        dag.connect(
            input2_id,
            input2_node.output_ports[0].id,
            mix_id,
            mix_node.input_ports[1].id,
            1.0,
        )
        .unwrap();

        dag.connect(
            mix_id,
            mix_node.output_ports[0].id,
            output_id,
            output_node.input_ports[0].id,
            1.0,
        )
        .unwrap();

        let mut input = AudioBlock::new(2, 256, 48000.0);
        for ch in 0..2 {
            for frame in 0..256 {
                input.samples[ch][frame] = 0.5;
            }
        }

        let output = dag.process(input).unwrap();

        for ch in 0..2 {
            for frame in 0..256 {
                assert!((output.samples[ch][frame] - 1.0).abs() < 0.0001);
            }
        }
    }
}
