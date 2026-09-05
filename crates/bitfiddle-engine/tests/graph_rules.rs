//! Graph construction tests: topological order, cycle rejection, Clock
//! single-source enforcement, input-sync groups (PRD §5.3, §6.2, §6.7).

use std::collections::{BTreeMap, HashMap};

use bitfiddle_engine::document::*;
use bitfiddle_engine::graph::{build_graph, would_create_cycle, GraphError};
use bitfiddle_engine::modules::registry::{builtin_spec, ModuleSpec};
use bitfiddle_engine::signal::SignalType;
use uuid::Uuid;

fn module(name: &str, type_id: &str, x: i64) -> ModuleInstance {
    let spec = builtin_spec(type_id).unwrap();
    let mut inputs = BTreeMap::new();
    for p in &spec.inputs {
        inputs.insert(p.id.clone(), InputState::default_for(p.signal));
    }
    ModuleInstance {
        id: Uuid::new_v4(),
        name: name.to_string(),
        type_id: type_id.to_string(),
        type_version: "2.0.0".to_string(),
        abi: Abi::Builtin2,
        state_version: 1,
        flavor: "Vanilla".to_string(),
        position: GridPoint { x, y: 0 },
        bypassed: false,
        input_ui: BTreeMap::new(),
        inputs,
        state: ModuleState {
            parameters: serde_json::Map::new(),
            custom: serde_json::Map::new(),
        },
    }
}

fn doc(modules: Vec<ModuleInstance>, wires: Vec<Wire>, input_sync: Vec<InputSync>) -> RackDocument {
    RackDocument {
        format: "bitfiddle-rack".into(),
        format_version: 2,
        app_version: "2.0.0".into(),
        rack: RackMetadata {
            id: Uuid::new_v4(),
            name: "Test".into(),
            revision: 1,
            created_at: "2026-01-01T00:00:00Z".into(),
            modified_at: "2026-01-01T00:00:00Z".into(),
        },
        engine: EngineConfig {
            sample_rate: 48000,
            block_size: 128,
            default_device_id: None,
        },
        view: View {
            pan: Pan { x: 0.0, y: 0.0 },
            zoom: 1.0,
            selected: vec![],
        },
        modules,
        wires,
        input_sync,
        macros: vec![],
    }
}

fn wire(src: &ModuleInstance, sport: &str, dst: &ModuleInstance, dport: &str, signal: SignalType, order: u32) -> Wire {
    Wire {
        id: Uuid::new_v4(),
        signal,
        source: Endpoint {
            module: src.id,
            port: sport.to_string(),
        },
        target: Endpoint {
            module: dst.id,
            port: dport.to_string(),
        },
        order,
        waypoints: vec![],
    }
}

fn specs_for(d: &RackDocument) -> HashMap<Uuid, ModuleSpec> {
    d.modules
        .iter()
        .map(|m| (m.id, builtin_spec(&m.type_id).unwrap()))
        .collect()
}

#[test]
fn topological_order_respects_wires() {
    let osc = module("Osc", "app.oscillator", 0);
    let vol = module("Vol", "app.volume", 8);
    let out = module("Out", "app.audio_output", 16);
    let w1 = wire(&osc, "audio_out", &vol, "audio_in", SignalType::Audio, 0);
    let w2 = wire(&vol, "audio_out", &out, "audio_in", SignalType::Audio, 1);
    // Deliberately declare modules in reverse order.
    let d = doc(vec![out.clone(), vol.clone(), osc.clone()], vec![w1, w2], vec![]);
    let g = build_graph(&d, &specs_for(&d)).unwrap();
    let pos = |id| g.execution_order.iter().position(|x| *x == id).unwrap();
    assert!(pos(osc.id) < pos(vol.id));
    assert!(pos(vol.id) < pos(out.id));
}

#[test]
fn cycle_is_rejected() {
    let a = module("A", "app.volume", 0);
    let b = module("B", "app.volume", 8);
    let w1 = wire(&a, "audio_out", &b, "audio_in", SignalType::Audio, 0);
    let w2 = wire(&b, "audio_out", &a, "audio_in", SignalType::Audio, 1);
    let d = doc(vec![a, b], vec![w1, w2], vec![]);
    let err = build_graph(&d, &specs_for(&d)).unwrap_err();
    assert!(matches!(err, GraphError::Cycle { .. }));
}

#[test]
fn would_create_cycle_detects_without_mutation() {
    let a = module("A", "app.volume", 0);
    let b = module("B", "app.volume", 8);
    let w1 = wire(&a, "audio_out", &b, "audio_in", SignalType::Audio, 0);
    let d = doc(vec![a.clone(), b.clone()], vec![w1], vec![]);
    let specs = specs_for(&d);
    assert!(would_create_cycle(
        &d,
        &specs,
        &Endpoint {
            module: b.id,
            port: "audio_out".into()
        },
        &Endpoint {
            module: a.id,
            port: "audio_in".into()
        },
    ));
    // Forward direction stays legal (parallel wire).
    assert!(!would_create_cycle(
        &d,
        &specs,
        &Endpoint {
            module: a.id,
            port: "audio_out".into()
        },
        &Endpoint {
            module: b.id,
            port: "audio_in".into()
        },
    ));
    assert_eq!(d.wires.len(), 1); // no mutation
}

#[test]
fn clock_input_rejects_second_source() {
    let c1 = module("Clock 1", "app.clock", 0);
    let c2 = module("Clock 2", "app.clock", 8);
    let target = module("Clock 3", "app.clock", 16);
    let w1 = wire(&c1, "clock_out", &target, "rate", SignalType::Clock, 0);
    let w2 = wire(&c2, "clock_out", &target, "rate", SignalType::Clock, 1);
    let d = doc(vec![c1, c2, target], vec![w1, w2], vec![]);
    let err = build_graph(&d, &specs_for(&d)).unwrap_err();
    assert!(matches!(err, GraphError::ClockMultipleSources { .. }));
}

#[test]
fn clock_single_source_through_sync_group_rejected() {
    // Two synced clock inputs receiving one source each => two sources for
    // the group, which must be rejected.
    let c1 = module("Clock 1", "app.clock", 0);
    let c2 = module("Clock 2", "app.clock", 8);
    let t1 = module("Target 1", "app.clock", 16);
    let t2 = module("Target 2", "app.clock", 24);
    let w1 = wire(&c1, "clock_out", &t1, "rate", SignalType::Clock, 0);
    let w2 = wire(&c2, "clock_out", &t2, "rate", SignalType::Clock, 1);
    let sync = InputSync {
        id: Uuid::new_v4(),
        signal: SignalType::Clock,
        a: Endpoint {
            module: t1.id,
            port: "rate".into(),
        },
        b: Endpoint {
            module: t2.id,
            port: "rate".into(),
        },
        waypoints: vec![],
    };
    let d = doc(vec![c1, c2, t1, t2], vec![w1, w2], vec![sync]);
    let err = build_graph(&d, &specs_for(&d)).unwrap_err();
    assert!(matches!(err, GraphError::ClockMultipleSources { .. }));
}

#[test]
fn sync_group_shares_sources_across_members() {
    let osc = module("Osc", "app.oscillator", 0);
    let v1 = module("Vol 1", "app.volume", 8);
    let v2 = module("Vol 2", "app.volume", 16);
    let w = wire(&osc, "audio_out", &v1, "audio_in", SignalType::Audio, 0);
    let sync = InputSync {
        id: Uuid::new_v4(),
        signal: SignalType::Audio,
        a: Endpoint {
            module: v1.id,
            port: "audio_in".into(),
        },
        b: Endpoint {
            module: v2.id,
            port: "audio_in".into(),
        },
        waypoints: vec![],
    };
    let d = doc(vec![osc.clone(), v1.clone(), v2.clone()], vec![w], vec![sync]);
    let g = build_graph(&d, &specs_for(&d)).unwrap();
    // Both endpoints share one representative with one source list.
    let e1 = Endpoint {
        module: v1.id,
        port: "audio_in".into(),
    };
    let e2 = Endpoint {
        module: v2.id,
        port: "audio_in".into(),
    };
    assert_eq!(g.sync_representative[&e1], g.sync_representative[&e2]);
    let rep = &g.sync_representative[&e1];
    assert_eq!(g.input_sources[rep].len(), 1);
    // Both consumer modules execute after the source.
    let pos = |id| g.execution_order.iter().position(|x| *x == id).unwrap();
    assert!(pos(osc.id) < pos(v1.id));
    assert!(pos(osc.id) < pos(v2.id));
}

#[test]
fn cycle_through_sync_group_is_rejected() {
    // osc -> v1(audio_in), v1 synced with v2, v2.audio_out -> osc? Osc has no
    // audio input, so use three volumes: a->b, b synced c, c->a.
    let a = module("A", "app.volume", 0);
    let b = module("B", "app.volume", 8);
    let c = module("C", "app.volume", 16);
    let w1 = wire(&a, "audio_out", &b, "audio_in", SignalType::Audio, 0);
    let w2 = wire(&c, "audio_out", &a, "audio_in", SignalType::Audio, 1);
    // Syncing b and c's inputs makes the wire into b also feed c, creating
    // a -> c -> a.
    let sync = InputSync {
        id: Uuid::new_v4(),
        signal: SignalType::Audio,
        a: Endpoint {
            module: b.id,
            port: "audio_in".into(),
        },
        b: Endpoint {
            module: c.id,
            port: "audio_in".into(),
        },
        waypoints: vec![],
    };
    let d = doc(vec![a, b, c], vec![w1, w2], vec![sync]);
    let err = build_graph(&d, &specs_for(&d)).unwrap_err();
    assert!(matches!(err, GraphError::Cycle { .. }));
}

#[test]
fn signal_mismatch_is_rejected() {
    let osc = module("Osc", "app.oscillator", 0);
    let adsr = module("Env", "app.adsr", 8);
    // audio out -> gate input: illegal.
    let w = wire(&osc, "audio_out", &adsr, "gate", SignalType::Audio, 0);
    let d = doc(vec![osc, adsr], vec![w], vec![]);
    let err = build_graph(&d, &specs_for(&d)).unwrap_err();
    assert!(matches!(err, GraphError::SignalMismatch { .. }));
}

#[test]
fn stable_order_is_deterministic_across_builds() {
    let osc = module("Osc", "app.oscillator", 0);
    let n1 = module("N1", "app.noise", 8);
    let n2 = module("N2", "app.noise", 16);
    let out = module("Out", "app.audio_output", 24);
    let w1 = wire(&osc, "audio_out", &out, "audio_in", SignalType::Audio, 0);
    let d = doc(vec![osc, n1, n2, out], vec![w1], vec![]);
    let specs = specs_for(&d);
    let g1 = build_graph(&d, &specs).unwrap();
    let g2 = build_graph(&d, &specs).unwrap();
    assert_eq!(g1.execution_order, g2.execution_order);
}
