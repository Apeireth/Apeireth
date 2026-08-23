use std::collections::{VecDeque, HashSet, HashMap};


#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FactNode(pub String);

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: FactNode,
    pub to: FactNode,
    pub weight: f64,
}

#[derive(Default, Clone)]
pub struct CausalGraph {
    nodes: HashSet<FactNode>,
    edges: Vec<Edge>,
    freq_map: HashMap<FactNode, usize>,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: FactNode) {
        self.nodes.insert(node.clone());
        *self.freq_map.entry(node).or_insert(0) += 1;
    }
    
    pub fn add_edge(&mut self, edge: Edge) {
        self.add_node(edge.from.clone());
        self.add_node(edge.to.clone());
        self.edges.push(edge);
    }

    pub fn intrinsic_residual_anchor_gain(&self, node: &FactNode) -> f64 {
        // entity inverse-frequency specificity
        let freq = self.freq_map.get(node).copied().unwrap_or(1);
        1.0 / (freq as f64).ln().max(1.0)
    }

    pub fn get_outgoing_edges(&self, from: &FactNode) -> Vec<&Edge> {
        self.edges.iter().filter(|e| &e.from == from).collect()
    }

    pub fn crawl(&self, start: &FactNode, max_depth: usize, budget: usize) -> Vec<FactNode> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((start.clone(), 0));
        visited.insert(start.clone());

        while let Some((node, depth)) = queue.pop_front() {
            if result.len() >= budget {
                break;
            }
            
            result.push(node.clone());
            
            if depth >= max_depth {
                continue;
            }
            
            for edge in &self.edges {
                if edge.from == node && !visited.contains(&edge.to) {
                    visited.insert(edge.to.clone());
                    queue.push_back((edge.to.clone(), depth + 1));
                } else if edge.to == node && !visited.contains(&edge.from) { // Bidirectional
                    visited.insert(edge.from.clone());
                    queue.push_back((edge.from.clone(), depth + 1));
                }
            }
        }
        result
    }
}

pub struct MctsNode {
    pub fact: FactNode,
    pub visits: usize,
    pub total_reward: f64,
    pub children: Vec<MctsNode>,
}

impl MctsNode {
    pub fn new(fact: FactNode) -> Self {
        Self {
            fact,
            visits: 0,
            total_reward: 0.0,
            children: Vec::new(),
        }
    }

    pub fn ucb1_score(&self, parent_visits: usize, exploration_const: f64) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let q = self.total_reward / self.visits as f64;
        let u = exploration_const * ((parent_visits as f64).ln() / self.visits as f64).sqrt();
        q + u
    }
}

pub struct MctsCausalSimulator {
    graph: CausalGraph,
    exploration_const: f64,
}

impl MctsCausalSimulator {
    pub fn new(graph: CausalGraph) -> Self {
        Self {
            graph,
            exploration_const: 1.414,
        }
    }

    pub fn simulate(&self, start: &FactNode, iterations: usize) -> Vec<FactNode> {
        let mut root = MctsNode::new(start.clone());

        // 1. Initial expansion from root
        let outgoing = self.graph.get_outgoing_edges(start);
        for edge in outgoing {
            root.children.push(MctsNode::new(edge.to.clone()));
        }

        if root.children.is_empty() {
            return vec![start.clone()];
        }

        // 2. Run MCTS iterations (Selection -> Rollout -> Backpropagation)
        for _ in 0..iterations {
            // Selection with UCB1
            let mut best_child_idx = 0;
            let mut best_score = -1.0;

            for (idx, child) in root.children.iter().enumerate() {
                let score = child.ucb1_score(root.visits + 1, self.exploration_const);
                if score > best_score {
                    best_score = score;
                    best_child_idx = idx;
                }
            }

            // Rollout reward simulation
            let chosen_node = &root.children[best_child_idx].fact;
            let specificity = self.graph.intrinsic_residual_anchor_gain(chosen_node);
            let reward = specificity * 1.0;

            // Backpropagation
            root.visits += 1;
            root.total_reward += reward;
            root.children[best_child_idx].visits += 1;
            root.children[best_child_idx].total_reward += reward;
        }

        // Return path with highest visited child
        let mut path = vec![start.clone()];
        if let Some(best) = root.children.iter().max_by_key(|c| c.visits) {
            path.push(best.fact.clone());
        }
        path
    }
}

