//! End-to-end check exercising the full first-checkpoint loop:
//!
//! 1. Dial criome over UDS
//! 2. Handshake exchange (mentci-lib's driver)
//! 3. Auto-query on connect (empty cache initially)
//! 4. Assert a Graph + a Node + an Edge directly via the wire
//! 5. Re-query — cache populated
//! 6. Print what the GraphsNav view shows
//!
//! Run criome-daemon first, then:
//!
//! ```text
//! cargo run --example handshake
//! ```

use mentci_lib::connection::driver::{spawn_driver, DaemonRole, DriverCmd};
use mentci_lib::WorkbenchState;
use signal::{
    AssertOperation, AuthProof, Body, Edge, EdgeQuery, Frame, Graph, GraphQuery, Node,
    NodeQuery, PatternField, QueryOperation, RelationKind, Request, Slot,
};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let socket_path: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/criome.sock".into())
        .into();

    let mut workbench = WorkbenchState::new(Slot::from(0u64));

    let runtime_handle = tokio::runtime::Handle::current();
    let mut handle = spawn_driver(&runtime_handle, socket_path, DaemonRole::Criome);

    // ── Phase 1: connect + auto-queries ────────────────────
    drain_into_model(&mut handle, &mut workbench, Duration::from_millis(500)).await;

    println!("---");
    println!("After connect (initial cache):");
    summarise(&workbench);

    // ── Phase 2: assert some test records ──────────────────
    let asserts = [
        AssertOperation::Graph(Graph {
            title: "Echo Pipeline".to_string(),
            nodes: vec![],
            edges: vec![],
            subgraphs: vec![],
        }),
        AssertOperation::Graph(Graph {
            title: "Build Defs".to_string(),
            nodes: vec![],
            edges: vec![],
            subgraphs: vec![],
        }),
        AssertOperation::Node(Node { name: "ticks".to_string() }),
        AssertOperation::Node(Node { name: "double".to_string() }),
        AssertOperation::Node(Node { name: "stdout".to_string() }),
        AssertOperation::Edge(Edge {
            from: Slot::from(1024u64),
            to: Slot::from(1025u64),
            kind: RelationKind::Flow,
        }),
    ];

    for op in asserts {
        let frame = Frame {
            principal_hint: Some(Slot::from(0u64)),
            auth_proof: Some(AuthProof::SingleOperator),
            body: Body::Request(Request::Assert(op)),
        };
        let _ = handle.cmds_tx.send(DriverCmd::SendFrame(frame));
    }
    drain_into_model(&mut handle, &mut workbench, Duration::from_millis(500)).await;

    // ── Phase 3: re-query to refresh the cache ─────────────
    let queries = [
        QueryOperation::Graph(GraphQuery { title: PatternField::Wildcard }),
        QueryOperation::Node(NodeQuery { name: PatternField::Wildcard }),
        QueryOperation::Edge(EdgeQuery {
            from: PatternField::Wildcard,
            to: PatternField::Wildcard,
            kind: PatternField::Wildcard,
        }),
    ];
    for op in queries {
        let frame = Frame {
            principal_hint: Some(Slot::from(0u64)),
            auth_proof: Some(AuthProof::SingleOperator),
            body: Body::Request(Request::Query(op)),
        };
        let _ = handle.cmds_tx.send(DriverCmd::SendFrame(frame));
    }
    drain_into_model(&mut handle, &mut workbench, Duration::from_millis(500)).await;

    println!("---");
    println!("After Assert + re-query:");
    summarise(&workbench);

    let _ = handle.cmds_tx.send(DriverCmd::Disconnect);
}

/// Drain events for `dur` while feeding them through the
/// model. Outbound Cmds the model produces get sent back
/// through the driver.
async fn drain_into_model(
    handle: &mut mentci_lib::connection::driver::ConnectionHandle,
    workbench: &mut WorkbenchState,
    dur: Duration,
) {
    let deadline = tokio::time::Instant::now() + dur;
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => break,
            ev = handle.events_rx.recv() => match ev {
                None => break,
                Some(ev) => {
                    println!("event: {ev:?}");
                    let cmds = workbench.on_engine_event(ev);
                    for cmd in cmds {
                        if let mentci_lib::Cmd::SendCriome { frame } = cmd {
                            let _ = handle.cmds_tx.send(DriverCmd::SendFrame(frame));
                        }
                    }
                }
            }
        }
    }
}

fn summarise(workbench: &WorkbenchState) {
    let view = workbench.view();
    println!("GraphsNav:");
    if view.graphs_nav.graphs.is_empty() {
        println!("  (empty)");
    } else {
        for entry in &view.graphs_nav.graphs {
            println!("  • {} ({:?})", entry.display_name, entry.kind);
        }
    }
    println!(
        "ModelCache: {} graphs · {} nodes · {} edges",
        workbench.cache.graphs.len(),
        workbench.cache.nodes.len(),
        workbench.cache.edges.len(),
    );
}
