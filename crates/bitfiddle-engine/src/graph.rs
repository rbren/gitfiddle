//! Typed DAG: port declarations, input-sync group collapse, cycle detection,
//! and stable topological ordering (PRD §5.3, §6.7).

use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

use crate::document::{Endpoint, RackDocument};
use crate::modules::registry::ModuleSpec;
use crate::signal::SignalType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortDecl {
    pub id: String,
    pub name: String,
    pub signal: SignalType,
    pub order: u32,
    pub is_input: bool,
}

/// A resolved, validated graph ready for execution.
#[derive(Debug)]
pub struct Graph {
    /// Module UUIDs in stable topological execution order.
    pub execution_order: Vec<Uuid>,
    /// For each input endpoint, the representative endpoint of its sync group.
    pub sync_representative: HashMap<Endpoint, Endpoint>,
    /// For each representative input endpoint, source wires in saved order.
    pub input_sources: HashMap<Endpoint, Vec<WireRef>>,
    /// Members of each sync group keyed by representative.
    pub sync_members: HashMap<Endpoint, Vec<Endpoint>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireRef {
    pub id: Uuid,
    pub source: Endpoint,
    pub order: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("cycle detected through modules: {path}")]
    Cycle { path: String },
    #[error("clock input group has more than one source: {input}")]
    ClockMultipleSources { input: String },
    #[error("unknown module referenced: {0}")]
    UnknownModule(Uuid),
    #[error("unknown port {port} on module {module}")]
    UnknownPort { module: Uuid, port: String },
    #[error(
        "signal type mismatch on wire {wire}: source {source_signal:?} target {target_signal:?}"
    )]
    SignalMismatch {
        wire: Uuid,
        source_signal: SignalType,
        target_signal: SignalType,
    },
    #[error("input-sync signal type mismatch: {a:?} vs {b:?}")]
    SyncSignalMismatch { a: SignalType, b: SignalType },
    #[error("wire targets an output or sources an input: {0}")]
    WrongDirection(Uuid),
}

/// Build and validate a graph from a document plus resolved module specs.
pub fn build_graph(
    doc: &RackDocument,
    specs: &HashMap<Uuid, ModuleSpec>,
) -> Result<Graph, GraphError> {
    // Index port declarations.
    let mut inputs: HashMap<(Uuid, &str), SignalType> = HashMap::new();
    let mut outputs: HashMap<(Uuid, &str), SignalType> = HashMap::new();
    for m in &doc.modules {
        let spec = specs.get(&m.id).ok_or(GraphError::UnknownModule(m.id))?;
        for p in &spec.inputs {
            inputs.insert((m.id, p.id.as_str()), p.signal);
        }
        for p in &spec.outputs {
            outputs.insert((m.id, p.id.as_str()), p.signal);
        }
    }

    let check_input = |e: &Endpoint| -> Result<SignalType, GraphError> {
        inputs
            .get(&(e.module, e.port.as_str()))
            .copied()
            .ok_or_else(|| GraphError::UnknownPort {
                module: e.module,
                port: e.port.clone(),
            })
    };
    let check_output = |e: &Endpoint| -> Result<SignalType, GraphError> {
        outputs
            .get(&(e.module, e.port.as_str()))
            .copied()
            .ok_or_else(|| GraphError::UnknownPort {
                module: e.module,
                port: e.port.clone(),
            })
    };

    // Union-find over synchronized inputs (PRD §6.7).
    let mut parent: HashMap<Endpoint, Endpoint> = HashMap::new();
    fn find(parent: &mut HashMap<Endpoint, Endpoint>, e: &Endpoint) -> Endpoint {
        let p = parent.get(e).cloned().unwrap_or_else(|| e.clone());
        if &p == e {
            return p;
        }
        let root = find(parent, &p);
        parent.insert(e.clone(), root.clone());
        root
    }

    for sync in &doc.input_sync {
        let sa = check_input(&sync.a)?;
        let sb = check_input(&sync.b)?;
        if sa != sb || sa != sync.signal {
            return Err(GraphError::SyncSignalMismatch { a: sa, b: sb });
        }
        let ra = find(&mut parent, &sync.a);
        let rb = find(&mut parent, &sync.b);
        if ra != rb {
            // Deterministic representative: lowest (module uuid, port).
            let (root, child) = if (ra.module, ra.port.as_str()) <= (rb.module, rb.port.as_str()) {
                (ra, rb)
            } else {
                (rb, ra)
            };
            parent.insert(child, root);
        }
    }

    // Validate wires and gather per-representative sources in saved order.
    let mut sync_representative: HashMap<Endpoint, Endpoint> = HashMap::new();
    let mut input_sources: HashMap<Endpoint, Vec<WireRef>> = HashMap::new();
    let mut sync_members: HashMap<Endpoint, Vec<Endpoint>> = HashMap::new();

    let mut all_inputs: Vec<Endpoint> = inputs
        .keys()
        .map(|(m, p)| Endpoint {
            module: *m,
            port: (*p).to_string(),
        })
        .collect();
    all_inputs.sort_by(|a, b| (a.module, &a.port).cmp(&(b.module, &b.port)));
    for e in &all_inputs {
        let rep = find(&mut parent, e);
        sync_representative.insert(e.clone(), rep.clone());
        sync_members.entry(rep).or_default().push(e.clone());
    }

    let mut wires_sorted: Vec<&crate::document::Wire> = doc.wires.iter().collect();
    wires_sorted.sort_by_key(|w| w.order);
    for w in &wires_sorted {
        let st_out = check_output(&w.source)?;
        let st_in = check_input(&w.target)?;
        if st_out != st_in || st_out != w.signal {
            return Err(GraphError::SignalMismatch {
                wire: w.id,
                source_signal: st_out,
                target_signal: st_in,
            });
        }
        let rep = sync_representative
            .get(&w.target)
            .cloned()
            .unwrap_or_else(|| w.target.clone());
        input_sources.entry(rep).or_default().push(WireRef {
            id: w.id,
            source: w.source.clone(),
            order: w.order,
        });
    }

    // Clock groups allow at most one source (PRD §6.2, §6.7).
    for (rep, sources) in &input_sources {
        if let Ok(SignalType::Clock) = check_input(rep) {
            if sources.len() > 1 {
                return Err(GraphError::ClockMultipleSources {
                    input: format!("{}.{}", rep.module, rep.port),
                });
            }
        }
    }

    // Build module-level adjacency after collapsing sync groups: a wire into
    // any group member feeds every member's module.
    let mut edges: BTreeMap<Uuid, HashSet<Uuid>> = BTreeMap::new();
    let mut indegree: BTreeMap<Uuid, usize> = BTreeMap::new();
    let mut doc_order: HashMap<Uuid, usize> = HashMap::new();
    for (i, m) in doc.modules.iter().enumerate() {
        edges.entry(m.id).or_default();
        indegree.entry(m.id).or_insert(0);
        doc_order.insert(m.id, i);
    }
    for (rep, sources) in &input_sources {
        let consumers: Vec<Uuid> = sync_members
            .get(rep)
            .map(|ms| ms.iter().map(|e| e.module).collect())
            .unwrap_or_else(|| vec![rep.module]);
        for w in sources {
            for c in &consumers {
                if w.source.module != *c && edges.get_mut(&w.source.module).unwrap().insert(*c) {
                    *indegree.get_mut(c).unwrap() += 1;
                }
            }
        }
    }

    // Kahn's algorithm with stable document-order tie-breaking.
    let mut ready: Vec<Uuid> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| *id)
        .collect();
    ready.sort_by_key(|id| doc_order[id]);
    let mut order: Vec<Uuid> = Vec::with_capacity(doc.modules.len());
    let mut indeg = indegree.clone();
    while let Some(&next) = ready.iter().min_by_key(|id| doc_order[*id]) {
        ready.retain(|id| *id != next);
        order.push(next);
        for succ in &edges[&next] {
            let d = indeg.get_mut(succ).unwrap();
            *d -= 1;
            if *d == 0 {
                ready.push(*succ);
            }
        }
    }
    if order.len() != doc.modules.len() {
        let remaining: Vec<String> = indeg
            .iter()
            .filter(|(_, d)| **d > 0)
            .map(|(id, _)| {
                doc.modules
                    .iter()
                    .find(|m| m.id == *id)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| id.to_string())
            })
            .collect();
        return Err(GraphError::Cycle {
            path: remaining.join(" -> "),
        });
    }

    Ok(Graph {
        execution_order: order,
        sync_representative,
        input_sources,
        sync_members,
    })
}

/// Check whether adding `source -> target` would create a cycle, without
/// mutating the document (PRD §5.3).
pub fn would_create_cycle(
    doc: &RackDocument,
    specs: &HashMap<Uuid, ModuleSpec>,
    source: &Endpoint,
    target: &Endpoint,
) -> bool {
    let mut candidate = doc.clone();
    let max_order = candidate.wires.iter().map(|w| w.order).max().unwrap_or(0);
    let signal = specs
        .get(&source.module)
        .and_then(|s| s.outputs.iter().find(|p| p.id == source.port))
        .map(|p| p.signal);
    let Some(signal) = signal else { return false };
    candidate.wires.push(crate::document::Wire {
        id: Uuid::new_v4(),
        signal,
        source: source.clone(),
        target: target.clone(),
        order: max_order + 1,
        waypoints: Vec::new(),
    });
    matches!(
        build_graph(&candidate, specs),
        Err(GraphError::Cycle { .. })
    )
}
