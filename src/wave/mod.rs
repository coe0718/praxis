//! Wave execution — group dependency-aware work into parallel waves.
//!
//! Instead of spawning parallelism ad hoc, wave execution performs a
//! topological sort over a dependency graph and groups work into sequential
//! **waves**.  Within a wave every node is independent (no intra-wave
//! dependencies), so all nodes in a wave could theoretically execute in
//! parallel.  Across waves, ordering is guaranteed by the dependency graph.
//!
//! # Terminology
//!
//! - **Node** — a unit of work with an ID and a list of dependency IDs.
//! - **Wave** — a set of nodes whose dependencies are all satisfied by
//!   previous waves.
//! - **Graph** — the full set of nodes; each node's `deps` must reference
//!   other node IDs in the same graph.
//!
//! # Example
//!
//! ```text
//! A ──► C ──► E
//!             ▲
//! B ──► D ────┘
//! ```
//!
//! Wave 0: [A, B]  (no deps)
//! Wave 1: [C, D]  (deps: A and B respectively)
//! Wave 2: [E]     (deps: C and D)

use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

// ── Node ──────────────────────────────────────────────────────────────────────

/// A single unit of work in the wave graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveNode {
    /// Unique identifier within the graph.
    pub id: String,
    /// Human-readable description of the work.
    pub description: String,
    /// IDs of nodes that must complete before this node can run.
    #[serde(default)]
    pub deps: Vec<String>,
}

impl WaveNode {
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            deps: Vec::new(),
        }
    }

    pub fn with_deps(mut self, deps: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.deps = deps.into_iter().map(|d| d.into()).collect();
        self
    }
}

// ── Graph ─────────────────────────────────────────────────────────────────────

/// A dependency graph of wave nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WaveGraph {
    pub nodes: Vec<WaveNode>,
}

impl WaveGraph {
    pub fn new(nodes: Vec<WaveNode>) -> Self {
        Self { nodes }
    }

    /// Topologically sort the graph into sequential waves.
    ///
    /// Returns `Err` if the graph contains cycles or unknown dependency IDs.
    pub fn into_waves(self) -> Result<Vec<Vec<WaveNode>>> {
        let node_map: HashMap<String, WaveNode> = self
            .nodes
            .iter()
            .map(|n| (n.id.clone(), n.clone()))
            .collect();

        // Validate: all deps must reference known nodes.
        for node in &self.nodes {
            for dep in &node.deps {
                if !node_map.contains_key(dep) {
                    bail!(
                        "node {} depends on unknown node {}",
                        node.id,
                        dep
                    );
                }
            }
        }

        // Kahn's algorithm for topological sort into levels.
        let mut in_degree: HashMap<&str, usize> = self
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.deps.len()))
            .collect();

        // adjacency: node → nodes that depend on it
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            dependents.entry(node.id.as_str()).or_default();
            for dep in &node.deps {
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(node.id.as_str());
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();
        // Sort for deterministic output.
        let mut initial: Vec<&str> = queue.drain(..).collect();
        initial.sort_unstable();
        queue.extend(initial);

        let mut waves: Vec<Vec<WaveNode>> = Vec::new();
        let mut processed = 0;

        while !queue.is_empty() {
            // Collect all nodes currently in the queue as one wave.
            let wave_ids: Vec<&str> = queue.drain(..).collect();
            let mut wave: Vec<WaveNode> = wave_ids
                .iter()
                .map(|&id| node_map[id].clone())
                .collect();
            wave.sort_by(|a, b| a.id.cmp(&b.id));
            processed += wave.len();

            // Reduce in-degree for all dependents and enqueue newly-free nodes.
            let mut next: Vec<&str> = Vec::new();
            for id in &wave_ids {
                if let Some(deps_on) = dependents.get(id) {
                    for &dep_id in deps_on {
                        // Safety: dep_id was validated as a known node above.
                        let deg = in_degree
                            .get_mut(dep_id)
                            .ok_or_else(|| anyhow::anyhow!("internal: missing in-degree entry for node {dep_id}"))?;
                        *deg -= 1;
                        if *deg == 0 {
                            next.push(dep_id);
                        }
                    }
                }
            }
            next.sort_unstable();
            queue.extend(next);

            if !wave.is_empty() {
                waves.push(wave);
            }
        }

        if processed != self.nodes.len() {
            bail!("dependency graph contains a cycle");
        }

        Ok(waves)
    }
}

// ── Execution ─────────────────────────────────────────────────────────────────

/// The outcome of executing one node within a wave.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveNodeResult {
    pub wave_index: usize,
    pub node_id: String,
    pub outcome: WaveOutcome,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WaveOutcome {
    Success,
    Failed,
    Skipped,
}

/// Execute all waves in order, calling `executor` for each node.
///
/// If a node fails, all nodes in subsequent waves that depend on it (directly
/// or transitively) are marked `Skipped`.  Nodes in the same wave that are
/// independent still run.
///
/// `executor` receives the node and returns `Ok(detail)` on success or `Err`
/// on failure.
pub fn execute_waves<F>(graph: WaveGraph, mut executor: F) -> Result<Vec<WaveNodeResult>>
where
    F: FnMut(&WaveNode) -> Result<String>,
{
    let waves = graph.into_waves()?;
    let mut results = Vec::new();
    let mut failed_ids: HashSet<String> = HashSet::new();

    for (wave_index, wave) in waves.iter().enumerate() {
        for node in wave {
            // Skip if any dependency failed.
            let blocked = node
                .deps
                .iter()
                .any(|dep| failed_ids.contains(dep.as_str()));

            if blocked {
                results.push(WaveNodeResult {
                    wave_index,
                    node_id: node.id.clone(),
                    outcome: WaveOutcome::Skipped,
                    detail: format!(
                        "Skipped because a dependency failed: {}",
                        node.deps
                            .iter()
                            .filter(|d| failed_ids.contains(d.as_str()))
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
                failed_ids.insert(node.id.clone());
                continue;
            }

            match executor(node) {
                Ok(detail) => results.push(WaveNodeResult {
                    wave_index,
                    node_id: node.id.clone(),
                    outcome: WaveOutcome::Success,
                    detail,
                }),
                Err(e) => {
                    results.push(WaveNodeResult {
                        wave_index,
                        node_id: node.id.clone(),
                        outcome: WaveOutcome::Failed,
                        detail: e.to_string(),
                    });
                    failed_ids.insert(node.id.clone());
                }
            }
        }
    }

    Ok(results)
}

/// Format a wave execution summary for logging.
pub fn summarize_waves(results: &[WaveNodeResult]) -> String {
    let success = results.iter().filter(|r| r.outcome == WaveOutcome::Success).count();
    let failed = results.iter().filter(|r| r.outcome == WaveOutcome::Failed).count();
    let skipped = results.iter().filter(|r| r.outcome == WaveOutcome::Skipped).count();
    format!(
        "Wave execution: {}/{} succeeded, {} failed, {} skipped",
        success,
        results.len(),
        failed,
        skipped,
    )
}

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use super::{WaveGraph, WaveNode, WaveOutcome, execute_waves, summarize_waves};

    fn node(id: &str) -> WaveNode {
        WaveNode::new(id, format!("Work for {id}"))
    }

    fn node_deps(id: &str, deps: &[&str]) -> WaveNode {
        WaveNode::new(id, format!("Work for {id}")).with_deps(deps.iter().copied())
    }

    #[test]
    fn independent_nodes_form_single_wave() {
        let graph = WaveGraph::new(vec![node("a"), node("b"), node("c")]);
        let waves = graph.into_waves().unwrap();
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].len(), 3);
    }

    #[test]
    fn chain_produces_one_node_per_wave() {
        let graph = WaveGraph::new(vec![
            node("a"),
            node_deps("b", &["a"]),
            node_deps("c", &["b"]),
        ]);
        let waves = graph.into_waves().unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0][0].id, "a");
        assert_eq!(waves[1][0].id, "b");
        assert_eq!(waves[2][0].id, "c");
    }

    #[test]
    fn diamond_collapses_middle_into_one_wave() {
        // A → B, A → C, B → D, C → D
        let graph = WaveGraph::new(vec![
            node("a"),
            node_deps("b", &["a"]),
            node_deps("c", &["a"]),
            node_deps("d", &["b", "c"]),
        ]);
        let waves = graph.into_waves().unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0].len(), 1); // [a]
        assert_eq!(waves[1].len(), 2); // [b, c]
        assert_eq!(waves[2].len(), 1); // [d]
    }

    #[test]
    fn cycle_returns_error() {
        let graph = WaveGraph::new(vec![
            node_deps("a", &["b"]),
            node_deps("b", &["a"]),
        ]);
        assert!(graph.into_waves().is_err());
    }

    #[test]
    fn unknown_dep_returns_error() {
        let graph = WaveGraph::new(vec![node_deps("a", &["ghost"])]);
        assert!(graph.into_waves().is_err());
    }

    #[test]
    fn execute_waves_runs_all_and_collects_results() {
        let graph = WaveGraph::new(vec![
            node("a"),
            node_deps("b", &["a"]),
        ]);
        let results = execute_waves(graph, |n| Ok(format!("ran {}", n.id))).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.outcome == WaveOutcome::Success));
    }

    #[test]
    fn failed_node_causes_dependents_to_be_skipped() {
        let graph = WaveGraph::new(vec![
            node("a"),
            node_deps("b", &["a"]),
            node_deps("c", &["b"]),
        ]);
        let results = execute_waves(graph, |n| {
            if n.id == "a" {
                bail!("a failed")
            } else {
                Ok(format!("ran {}", n.id))
            }
        })
        .unwrap();

        assert_eq!(results[0].outcome, WaveOutcome::Failed);
        assert_eq!(results[1].outcome, WaveOutcome::Skipped);
        assert_eq!(results[2].outcome, WaveOutcome::Skipped);
    }

    #[test]
    fn sibling_in_same_wave_still_runs_when_unrelated_node_fails() {
        // a and b are independent; c depends on a only.
        let graph = WaveGraph::new(vec![
            node("a"),
            node("b"),
            node_deps("c", &["a"]),
        ]);
        let results = execute_waves(graph, |n| {
            if n.id == "a" {
                bail!("a failed")
            } else {
                Ok(format!("ran {}", n.id))
            }
        })
        .unwrap();

        let b = results.iter().find(|r| r.node_id == "b").unwrap();
        assert_eq!(b.outcome, WaveOutcome::Success);

        let c = results.iter().find(|r| r.node_id == "c").unwrap();
        assert_eq!(c.outcome, WaveOutcome::Skipped);
    }

    #[test]
    fn summarize_gives_correct_counts() {
        let graph = WaveGraph::new(vec![
            node("a"),
            node("b"),
            node_deps("c", &["a"]),
        ]);
        let results = execute_waves(graph, |n| {
            if n.id == "a" {
                bail!("fail")
            } else {
                Ok("ok".to_string())
            }
        })
        .unwrap();

        let summary = summarize_waves(&results);
        assert!(summary.contains("1/3 succeeded"));
        assert!(summary.contains("1 failed"));
        assert!(summary.contains("1 skipped"));
    }
}
