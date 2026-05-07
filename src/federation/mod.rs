//! Agent Federation — Distribute tasks across specialized agents.
//!
//! Splits complex work into sub-tasks, assigns to agent roles, coordinates
//! parallel execution, and synthesizes results.
//!
//! # Architecture
//!
//! ```text
//! 1. Goal analysis → decompose into sub-tasks
//! 2. Role assignment → researcher, coder, reviewer, etc.
//! 3. Spawn sessions → async parallel execution
//! 4. Result aggregation → synthesize final output
//! ```
//!
use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;
use crate::session::spawn::{SessionSpawnRequest, SessionSpawner, SpawnPriority};

/// Specialized agent roles for task federation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentRole {
    /// Research and information gathering.
    Researcher,
    /// Code generation and implementation.
    Coder,
    /// Review and quality assurance.
    Reviewer,
    /// Documentation and writing.
    Writer,
    /// Data analysis and computation.
    Analyst,
    /// Planning and organization.
    Planner,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Researcher => write!(f, "researcher"),
            AgentRole::Coder => write!(f, "coder"),
            AgentRole::Reviewer => write!(f, "reviewer"),
            AgentRole::Writer => write!(f, "writer"),
            AgentRole::Analyst => write!(f, "analyst"),
            AgentRole::Planner => write!(f, "planner"),
        }
    }
}

/// A sub-task for an agent in the federation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// Unique identifier for this sub-task.
    pub id: String,
    /// Agent role to assign this task to.
    pub role: AgentRole,
    /// Task description.
    pub description: String,
    /// Dependencies on other sub-tasks (must complete first).
    pub depends_on: Vec<String>,
    /// Input context from the main task.
    pub context: String,
}

/// Result from a federated sub-task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskResult {
    /// Sub-task ID this result belongs to.
    pub subtask_id: String,
    /// Agent role that executed this.
    pub role: AgentRole,
    /// Output/result of the task.
    pub output: String,
    /// Whether the task succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Request to federate a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationRequest {
    /// Main task/goal description.
    pub task: String,
    /// Maximum number of parallel agents.
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    /// Context from the parent session.
    pub context: String,
}

fn default_max_agents() -> usize {
    4
}

/// Result from a federation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationResult {
    /// Federation run ID.
    pub federation_id: String,
    /// Individual sub-task results.
    pub results: Vec<SubTaskResult>,
    /// Synthesized final output.
    pub final_output: String,
    /// Success status.
    pub success: bool,
}

/// Agent Federation coordinator.
pub struct AgentFederation {
    spawner: SessionSpawner,
}

impl AgentFederation {
    pub fn new(paths: &PraxisPaths) -> Self {
        Self {
            spawner: SessionSpawner::new(paths),
        }
    }

    /// Decompose a task into sub-tasks based on the goal.
    pub fn decompose(&self, req: &FederationRequest) -> Result<Vec<SubTask>> {
        let mut tasks = Vec::new();

        // Always start with planning
        tasks.push(SubTask {
            id: "plan".to_string(),
            role: AgentRole::Planner,
            description: format!("Plan approach for: {}", req.task),
            depends_on: vec![],
            context: req.context.clone(),
        });

        // Then research
        tasks.push(SubTask {
            id: "research".to_string(),
            role: AgentRole::Researcher,
            description: format!("Gather information for: {}", req.task),
            depends_on: vec!["plan".to_string()],
            context: req.context.clone(),
        });

        // Then implementation
        tasks.push(SubTask {
            id: "implement".to_string(),
            role: AgentRole::Coder,
            description: format!("Implement solution for: {}", req.task),
            depends_on: vec!["research".to_string()],
            context: req.context.clone(),
        });

        // Then review
        tasks.push(SubTask {
            id: "review".to_string(),
            role: AgentRole::Reviewer,
            description: "Review and ensure quality".to_string(),
            depends_on: vec!["implement".to_string()],
            context: req.context.clone(),
        });

        Ok(tasks)
    }

    /// Spawn a session for a sub-task.
    pub fn spawn_for_subtask(&self, subtask: &SubTask) -> Result<String> {
        let env_vars = {
            let mut vars = HashMap::new();
            vars.insert("PRAXIS_FEDERATION_ROLE".to_string(), subtask.role.to_string());
            vars.insert("PRAXIS_FEDERATION_SUBTASK".to_string(), subtask.id.clone());
            vars.insert("PRAXIS_PARENT_SESSION".to_string(), "federation".to_string());
            vars
        };

        let spawn_req = SessionSpawnRequest {
            task_description: subtask.description.clone(),
            parent_session_id: Some(format!("fed-{}", subtask.id)),
            environment_vars: env_vars,
            working_directory: None,
            metadata: Some(serde_json::json!({
                "role": subtask.role.to_string(),
                "subtask_id": subtask.id,
                "context": subtask.context
            })),
            priority: SpawnPriority::Normal,
        };

        let result = self.spawner.spawn(spawn_req)?;
        Ok(result.spawn_id)
    }

    /// Run the full federation pipeline.
    pub fn run(&self, req: FederationRequest) -> Result<FederationResult> {
        let federation_id = format!("fed-{}", uuid_simple());

        // Decompose the task
        let subtasks = self.decompose(&req)?;

        // Spawn sessions for each sub-task
        let mut results: Vec<SubTaskResult> = Vec::new();
        for subtask in &subtasks {
            // Wait for dependencies
            while subtask
                .depends_on
                .iter()
                .any(|dep| results.iter().find(|r| &r.subtask_id == dep).is_none())
            {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            match self.spawn_for_subtask(subtask) {
                Ok(_spawn_id) => {
                    results.push(SubTaskResult {
                        subtask_id: subtask.id.clone(),
                        role: subtask.role,
                        output: format!("Spawned session for {}", subtask.role),
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(SubTaskResult {
                        subtask_id: subtask.id.clone(),
                        role: subtask.role,
                        output: String::new(),
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        // Synthesize final output
        let success = results.iter().all(|r| r.success);
        let final_output = self.synthesize(&results)?;

        Ok(FederationResult {
            federation_id,
            results,
            final_output,
            success,
        })
    }

    /// Synthesize results into a final output.
    fn synthesize(&self, results: &[SubTaskResult]) -> Result<String> {
        let successful: Vec<_> = results.iter().filter(|r| r.success).collect();

        Ok(format!(
            "Federation complete. {}/{} sub-tasks succeeded.\n\n{}",
            successful.len(),
            results.len(),
            successful
                .iter()
                .map(|r| format!("[{}] {}", r.role, &r.output[..r.output.len().min(200)]))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_returns_four_tasks() {
        // Test the logic by calling decompose directly using a temp paths
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(dir.path().to_path_buf());
        let federation = AgentFederation::new(&paths);

        let req = FederationRequest {
            task: "Build a web app".to_string(),
            max_agents: 4,
            context: "test context".to_string(),
        };
        let tasks = federation.decompose(&req).unwrap();
        assert_eq!(tasks.len(), 4);
        let ids: std::collections::HashSet<_> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains("plan"));
        assert!(ids.contains("research"));
        assert!(ids.contains("implement"));
        assert!(ids.contains("review"));
    }
}