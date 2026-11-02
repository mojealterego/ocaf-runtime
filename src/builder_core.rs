//! Stage 0.50: bounded, typed and side-effect-free Agents Builder graph IR.
//!
//! This module validates a deliberately restricted JSON Code DSL shared with
//! NoCode graph editors. Compilation produces an inert plan and digest; it does
//! not execute tools, grant permissions, publish artifacts or authenticate a user.
//! The tenant argument MUST come from a verified ingress identity.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const SCHEMA_VERSION: u32 = 1;
const MAX_DSL_BYTES: usize = 128 * 1024;
const MAX_NODES: usize = 64;
const MAX_EDGES: usize = 256;
const MAX_PORTS: usize = 8;
const MAX_GRANTS: usize = 32;
const MAX_COST_MINOR: u64 = 1_000_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortType {
    Text,
    Json,
    Bytes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Input,
    Transform,
    Tool,
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub name: String,
    pub kind: PortType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderNode {
    pub id: String,
    pub kind: NodeKind,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub tool_name: Option<String>,
    pub max_cost_minor: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderEdge {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuilderProject {
    pub schema_version: u32,
    pub tenant_id: String,
    pub project_id: String,
    pub revision: u64,
    pub max_cost_minor: u64,
    pub grants: Vec<String>,
    pub nodes: Vec<BuilderNode>,
    pub edges: Vec<BuilderEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPlan {
    pub ordered_nodes: Vec<String>,
    pub requested_tools: Vec<String>,
    pub max_cost_minor: u64,
    pub project_sha256: String,
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum BuilderError {
    #[error("invalid schema, identifier, revision or node contract")]
    Invalid,
    #[error("authenticated tenant does not own this project")]
    Denied,
    #[error("invalid, cyclic, disconnected or ambiguous graph")]
    Graph,
    #[error("typed edge ports do not match")]
    TypeMismatch,
    #[error("missing, duplicate or excessive tool permissions")]
    Permission,
    #[error("declared effect cost exceeds project budget")]
    Budget,
    #[error("graph, DSL or cost exceeds a hard resource limit")]
    Limit,
    #[error("malformed or unsupported JSON Code DSL")]
    Serialization,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

fn valid_ports(ports: &[Port]) -> bool {
    if ports.len() > MAX_PORTS {
        return false;
    }
    let mut names = BTreeSet::new();
    ports
        .iter()
        .all(|port| valid_id(&port.name) && names.insert(&port.name))
}

fn reachable(start: &str, links: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![start.to_owned()];
    while let Some(id) = stack.pop() {
        if seen.insert(id.clone()) {
            if let Some(neighbors) = links.get(&id) {
                stack.extend(neighbors.iter().cloned());
            }
        }
    }
    seen
}

fn validate(
    project: &BuilderProject,
    trusted_tenant: &str,
) -> Result<(Vec<String>, Vec<String>), BuilderError> {
    if !valid_id(trusted_tenant) || project.tenant_id != trusted_tenant {
        return Err(BuilderError::Denied);
    }
    if project.schema_version != SCHEMA_VERSION
        || !valid_id(&project.project_id)
        || project.revision == 0
    {
        return Err(BuilderError::Invalid);
    }
    if project.max_cost_minor > MAX_COST_MINOR
        || project.nodes.len() > MAX_NODES
        || project.edges.len() > MAX_EDGES
        || project.grants.len() > MAX_GRANTS
    {
        return Err(BuilderError::Limit);
    }
    if project.nodes.len() < 2 {
        return Err(BuilderError::Invalid);
    }

    let mut granted = BTreeSet::new();
    for grant in &project.grants {
        if !valid_id(grant) || !granted.insert(grant.clone()) {
            return Err(BuilderError::Permission);
        }
    }

    let mut nodes = BTreeMap::new();
    let mut requested = BTreeSet::new();
    let mut declared_cost = 0_u64;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for node in &project.nodes {
        if !valid_id(&node.id)
            || !valid_ports(&node.inputs)
            || !valid_ports(&node.outputs)
            || nodes.insert(node.id.clone(), node).is_some()
        {
            return Err(BuilderError::Invalid);
        }
        match node.kind {
            NodeKind::Input => {
                if !node.inputs.is_empty()
                    || node.outputs.is_empty()
                    || node.tool_name.is_some()
                    || node.max_cost_minor != 0
                {
                    return Err(BuilderError::Invalid);
                }
                inputs.push(node.id.clone());
            }
            NodeKind::Output => {
                if node.inputs.is_empty()
                    || !node.outputs.is_empty()
                    || node.tool_name.is_some()
                    || node.max_cost_minor != 0
                {
                    return Err(BuilderError::Invalid);
                }
                outputs.push(node.id.clone());
            }
            NodeKind::Transform => {
                if node.inputs.is_empty()
                    || node.outputs.is_empty()
                    || node.tool_name.is_some()
                    || node.max_cost_minor != 0
                {
                    return Err(BuilderError::Invalid);
                }
            }
            NodeKind::Tool => {
                if node.inputs.is_empty() || node.outputs.is_empty() || node.max_cost_minor == 0 {
                    return Err(BuilderError::Invalid);
                }
                let name = node.tool_name.as_deref().ok_or(BuilderError::Invalid)?;
                if !valid_id(name) {
                    return Err(BuilderError::Invalid);
                }
                requested.insert(name.to_owned());
                declared_cost = declared_cost
                    .checked_add(node.max_cost_minor)
                    .ok_or(BuilderError::Limit)?;
            }
        }
    }
    if requested != granted {
        return Err(BuilderError::Permission);
    }
    if declared_cost > project.max_cost_minor {
        return Err(BuilderError::Budget);
    }

    let mut edges = BTreeSet::new();
    let mut incoming = BTreeSet::new();
    let mut degrees: BTreeMap<String, usize> = nodes.keys().map(|id| (id.clone(), 0)).collect();
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut parents: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for edge in &project.edges {
        if edge.from_node == edge.to_node || !edges.insert(edge.clone()) {
            return Err(BuilderError::Graph);
        }
        let source = nodes.get(&edge.from_node).ok_or(BuilderError::Graph)?;
        let target = nodes.get(&edge.to_node).ok_or(BuilderError::Graph)?;
        let source_port = source
            .outputs
            .iter()
            .find(|port| port.name == edge.from_port)
            .ok_or(BuilderError::Graph)?;
        let target_port = target
            .inputs
            .iter()
            .find(|port| port.name == edge.to_port)
            .ok_or(BuilderError::Graph)?;
        if source_port.kind != target_port.kind {
            return Err(BuilderError::TypeMismatch);
        }
        if !incoming.insert((edge.to_node.clone(), edge.to_port.clone())) {
            return Err(BuilderError::Graph);
        }
        let degree = degrees.get_mut(&edge.to_node).ok_or(BuilderError::Graph)?;
        *degree += 1;
        children
            .entry(edge.from_node.clone())
            .or_default()
            .push(edge.to_node.clone());
        parents
            .entry(edge.to_node.clone())
            .or_default()
            .push(edge.from_node.clone());
    }

    let mut ready: BTreeSet<String> = degrees
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut order = Vec::with_capacity(nodes.len());
    while let Some(id) = ready.pop_first() {
        if let Some(dependents) = children.get(&id) {
            for child in dependents {
                let degree = degrees.get_mut(child).ok_or(BuilderError::Graph)?;
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
        order.push(id);
    }
    if order.len() != nodes.len() {
        return Err(BuilderError::Graph);
    }
    if inputs.len() != 1 || outputs.len() != 1 {
        return Err(BuilderError::Invalid);
    }
    let from_input = reachable(&inputs[0], &children);
    let to_output = reachable(&outputs[0], &parents);
    if from_input.len() != nodes.len() || to_output.len() != nodes.len() {
        return Err(BuilderError::Graph);
    }

    Ok((order, requested.into_iter().collect()))
}

fn canonical(project: &BuilderProject, order: &[String]) -> BuilderProject {
    let mut normalized = project.clone();
    let rank: BTreeMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    normalized
        .nodes
        .sort_by_key(|node| rank.get(node.id.as_str()).copied().unwrap_or(usize::MAX));
    for node in &mut normalized.nodes {
        node.inputs.sort_by(|a, b| a.name.cmp(&b.name));
        node.outputs.sort_by(|a, b| a.name.cmp(&b.name));
    }
    normalized.edges.sort();
    normalized.grants.sort();
    normalized
}

/// Validate and compile an inert plan. This never authorizes or executes an effect.
pub fn compile(
    project: &BuilderProject,
    trusted_tenant: &str,
) -> Result<CompiledPlan, BuilderError> {
    let (ordered_nodes, requested_tools) = validate(project, trusted_tenant)?;
    let normalized = canonical(project, &ordered_nodes);
    let bytes = serde_json::to_vec(&normalized).map_err(|_| BuilderError::Serialization)?;
    if bytes.len() > MAX_DSL_BYTES {
        return Err(BuilderError::Limit);
    }
    let project_sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(CompiledPlan {
        ordered_nodes,
        requested_tools,
        max_cost_minor: project.max_cost_minor,
        project_sha256,
    })
}

/// Export the supported, canonical JSON Code DSL for a NoCode graph editor.
pub fn to_dsl(project: &BuilderProject, trusted_tenant: &str) -> Result<String, BuilderError> {
    let plan = compile(project, trusted_tenant)?;
    let normalized = canonical(project, &plan.ordered_nodes);
    let result =
        serde_json::to_string_pretty(&normalized).map_err(|_| BuilderError::Serialization)?;
    if result.len() > MAX_DSL_BYTES {
        return Err(BuilderError::Limit);
    }
    Ok(result)
}

/// Import strict JSON Code DSL. Unknown fields and unsupported schema fail closed.
pub fn from_dsl(source: &str, trusted_tenant: &str) -> Result<BuilderProject, BuilderError> {
    if source.len() > MAX_DSL_BYTES {
        return Err(BuilderError::Limit);
    }
    let project: BuilderProject =
        serde_json::from_str(source).map_err(|_| BuilderError::Serialization)?;
    let plan = compile(&project, trusted_tenant)?;
    Ok(canonical(&project, &plan.ordered_nodes))
}
