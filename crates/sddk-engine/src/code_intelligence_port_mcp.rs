//! Production MCP stdio adapter for the `CodeIntelligencePort`.
//!
//! Authority: `arch-spec-021` (IPB-001..012), ADR-0137.
//! Cycle: `p-63676b11dc0ef88f/aiw-s1-cognicode-real` (AIW-S1).
//!
//! Spawns the real `cognicode-mcp` binary over stdio (JSON-RPC,
//! one message per line on stdout; tracing logs go to stderr and
//! are consumed, not interpreted). Provider types never cross the
//! boundary: responses are parsed strictly into SDDK ADTs, and
//! any malformed payload fails closed.
//!
//! Known incidents (out of scope for AIW-S1, recorded separately):
//! - `tools/list` pagination cursor repeats entries (upstream bug).
//! - The editor HTTP endpoint config (`127.0.0.1:9847`) has no
//!   listener; this adapter uses stdio only.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use crate::code_intelligence_port::{
    AnalysisBasis, AnalysisResult, CapabilityProfile, CapabilitySnapshot, CodeIntelligencePort,
    CodeIntelligencePortError, DigestSha256, ImpactRequest, Observation, ObservationSet,
    ProviderKind, ProviderLifecycle, ScopeRequest,
};

/// Supported MCP protocol major. Mismatches fail closed
/// (`ProtocolMajorMismatch`), mirroring IPB-006.
pub const SUPPORTED_PROTOCOL_MAJOR: u32 = 2025;

/// Default wait for a JSON-RPC response.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

enum Reply {
    Message(serde_json::Value),
    #[allow(dead_code)] // wired when timeout handling lands in CC-S1+
    Timeout,
    Eof,
}

/// Handle to one spawned `cognicode-mcp` process.
pub struct CogniCodeMcpAdapter {
    child: Child,
    send: std::sync::Mutex<ChildStdin>,
    state: std::sync::Mutex<AdapterState>,
    snapshot: std::sync::Mutex<CapabilitySnapshot>,
    #[allow(dead_code)] // used by basis() -> AnalysisBasis (AIW-S2 surfacing)
    source_revision: String,
}

struct AdapterState {
    replies: Receiver<Reply>,
    next_id: u64,
    server_version: String,
}

impl CogniCodeMcpAdapter {
    /// Spawn the provider and run the MCP initialize handshake.
    ///
    /// `binary` is the path to `cognicode-mcp`; `cwd` is passed
    /// via `--cwd` (the analysis root). Fails closed on spawn or
    /// handshake failure, or on protocol-major mismatch.
    pub fn spawn(
        binary: &str,
        cwd: &str,
        source_revision: &str,
        timeout: Duration,
    ) -> Result<Self, CodeIntelligencePortError> {
        let mut child = Command::new(binary)
            .arg("--cwd")
            .arg(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| CodeIntelligencePortError::Unavailable)?;

        let stdin = child
            .stdin
            .take()
            .ok_or(CodeIntelligencePortError::Unavailable)?;
        let stdout = child
            .stdout
            .take()
            .ok_or(CodeIntelligencePortError::Unavailable)?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => match serde_json::from_str::<serde_json::Value>(&l) {
                        Ok(v) => {
                            if tx.send(Reply::Message(v)).is_err() {
                                return;
                            }
                        }
                        // Non-JSON lines on stdout are protocol
                        // violations but we stay tolerant: skip.
                        Err(_) => continue,
                    },
                    Err(_) => {
                        let _ = tx.send(Reply::Eof);
                        return;
                    }
                }
            }
            let _ = tx.send(Reply::Eof);
        });

        let adapter = Self {
            child,
            send: std::sync::Mutex::new(stdin),
            state: std::sync::Mutex::new(AdapterState {
                replies: rx,
                next_id: 1,
                server_version: String::new(),
            }),
            snapshot: std::sync::Mutex::new(CapabilitySnapshot::from_advertised(
                CapabilityProfile::StaticEnhanced,
                &[],
            )),
            source_revision: source_revision.to_string(),
        };
        adapter.initialize(timeout)?;
        Ok(adapter)
    }

    fn initialize(&self, timeout: Duration) -> Result<(), CodeIntelligencePortError> {
        let id = {
            let mut st = self.lock_state();
            let id = st.next_id;
            st.next_id += 1;
            id
        };
        let req = serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "sddk-engine", "version": env!("CARGO_PKG_VERSION")}
            }
        });
        self.send_request(&req)?;
        let reply = self.wait_reply(id, timeout)?;
        let major = protocol_major(reply.get("result"))?;
        if major != SUPPORTED_PROTOCOL_MAJOR {
            return Err(CodeIntelligencePortError::ProtocolMajorMismatch {
                provider_major: major,
                sddk_major: SUPPORTED_PROTOCOL_MAJOR,
            });
        }
        self.lock_state().server_version = reply["result"]["serverInfo"]["version"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let _ = self.notify("notifications/initialized");
        let analyzer_set = ["build_graph", "find_usages", "analyze_impact"];
        *self
            .snapshot
            .lock()
            .map_err(|_| CodeIntelligencePortError::Unavailable)? =
            CapabilitySnapshot::from_advertised(CapabilityProfile::StaticEnhanced, &analyzer_set);
        Ok(())
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, AdapterState> {
        self.state.lock().expect("adapter state poisoned")
    }

    fn next_id(&self) -> u64 {
        let mut st = self.lock_state();
        let id = st.next_id;
        st.next_id += 1;
        id
    }

    fn send_request(&self, req: &serde_json::Value) -> Result<(), CodeIntelligencePortError> {
        let mut line =
            serde_json::to_string(req).map_err(|_| CodeIntelligencePortError::Unavailable)?;
        line.push('\n');
        let mut stdin = self
            .send
            .lock()
            .map_err(|_| CodeIntelligencePortError::Unavailable)?;
        stdin
            .write_all(line.as_bytes())
            .map_err(|_| CodeIntelligencePortError::Unavailable)?;
        stdin
            .flush()
            .map_err(|_| CodeIntelligencePortError::Unavailable)
    }

    fn notify(&self, method: &str) -> Result<(), CodeIntelligencePortError> {
        let req = serde_json::json!({"jsonrpc": "2.0", "method": method});
        self.send_request(&req)
    }

    fn wait_reply(
        &self,
        id: u64,
        timeout: Duration,
    ) -> Result<serde_json::Value, CodeIntelligencePortError> {
        // Single outstanding request at a time: scan the channel
        // for our id; non-matching responses are dropped (strict
        // ordering per stdio contract).
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let now = std::time::Instant::now();
            if now >= deadline {
                return Err(CodeIntelligencePortError::Timeout);
            }
            let reply = {
                let st = self.lock_state();
                st.replies.recv_timeout(deadline - now)
            };
            match reply {
                Ok(Reply::Message(v)) => {
                    if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
                        if let Some(err) = v.get("error") {
                            let _ = err;
                            return Err(CodeIntelligencePortError::ProviderError);
                        }
                        return Ok(v);
                    }
                }
                Ok(Reply::Timeout) | Err(_) => return Err(CodeIntelligencePortError::Timeout),
                Ok(Reply::Eof) => return Err(CodeIntelligencePortError::Unavailable),
            }
        }
    }

    /// Call a tool and return the parsed JSON payload carried in
    /// `content[0].text`. Fails closed on missing text, invalid
    /// JSON, or `isError` responses.
    fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, CodeIntelligencePortError> {
        let id = self.next_id();
        let req = serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": name, "arguments": args}
        });
        self.send_request(&req)?;
        let reply = self.wait_reply(id, timeout)?;
        let result = reply
            .get("result")
            .ok_or(CodeIntelligencePortError::ProviderError)?;
        if result.get("isError").and_then(|b| b.as_bool()) == Some(true) {
            return Err(CodeIntelligencePortError::ProviderError);
        }
        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or(CodeIntelligencePortError::InvalidPayload)?;
        let text = content
            .first()
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .ok_or(CodeIntelligencePortError::InvalidPayload)?;
        serde_json::from_str(text).map_err(|_| CodeIntelligencePortError::InvalidPayload)
    }

    fn build_graph(&self, strategy: &str) -> Result<(), CodeIntelligencePortError> {
        let payload = self.call_tool(
            "build_graph",
            serde_json::json!({"strategy": strategy}),
            DEFAULT_TIMEOUT,
        )?;
        if payload.get("success").and_then(|b| b.as_bool()) == Some(true) {
            Ok(())
        } else {
            Err(CodeIntelligencePortError::ProviderError)
        }
    }

    /// Analysis basis for digest material. Currently consumed only by
    /// `digest_result`; retained for future AnalysisBasis surfacing
    /// (AIW-S2). Not dead code by intent.
    #[allow(dead_code)]
    fn basis(&self, scope: &str) -> AnalysisBasis {
        AnalysisBasis {
            provider_build: format!("cognicode-mcp/{}", self.lock_state().server_version.clone()),
            protocol_major: 2025,
            protocol_minor: 3,
            capability_snapshot: self.capabilities(),
            analyzer_set_digest: self.capabilities().analyzer_set_digest.clone(),
            source_revision: self.source_revision.clone(),
            request_scope: scope.to_string(),
        }
    }

    fn digest_result(
        &self,
        basis: &AnalysisBasis,
        units: &BTreeMap<String, Vec<Observation>>,
    ) -> DigestSha256 {
        let mut material = basis.provider_build.clone();
        material.push('|');
        material.push_str(&basis.source_revision);
        material.push('|');
        material.push_str(&basis.request_scope);
        material.push('|');
        for (unit, obs) in units {
            material.push_str(unit);
            for o in obs {
                material.push('\u{1}');
                material.push_str(&o.text);
            }
            material.push('\u{2}');
        }
        DigestSha256::of(material.as_bytes())
    }
}

impl Drop for CogniCodeMcpAdapter {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl CodeIntelligencePort for CogniCodeMcpAdapter {
    fn capabilities(&self) -> CapabilitySnapshot {
        self.snapshot.lock().expect("snapshot poisoned").clone()
    }

    fn lifecycle_state(&self) -> ProviderLifecycle {
        ProviderLifecycle::Ready
    }

    fn analyze_delta(
        &self,
        basis: &AnalysisBasis,
        _request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        let _ = basis;
        Err(CodeIntelligencePortError::Unsupported)
    }

    fn analyze_scope(
        &self,
        basis: &AnalysisBasis,
        request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        self.build_graph("full")?;
        let mut units = BTreeMap::new();
        for unit in request.added_units.iter().chain(&request.modified_units) {
            let payload = self.call_tool(
                "find_usages",
                serde_json::json!({"symbol_name": unit}),
                DEFAULT_TIMEOUT,
            )?;
            let text = serde_json::to_string(&payload)
                .map_err(|_| CodeIntelligencePortError::InvalidPayload)?;
            units.insert(unit.clone(), vec![Observation { text }]);
        }
        let digest = self.digest_result(basis, &units);
        Ok(AnalysisResult {
            digest,
            partial: false,
            observations: ObservationSet {
                provider_kind: ProviderKind::CogniCode,
                units,
                restart_observed: false,
            },
        })
    }

    fn analyze_impact(
        &self,
        basis: &AnalysisBasis,
        request: &ImpactRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError> {
        self.build_graph("full")?;
        let mut units = BTreeMap::new();
        for unit in &request.changed_units {
            let payload = self.call_tool(
                "analyze_impact",
                serde_json::json!({"symbol_name": unit}),
                DEFAULT_TIMEOUT,
            )?;
            let text = serde_json::to_string(&payload)
                .map_err(|_| CodeIntelligencePortError::InvalidPayload)?;
            units.insert(unit.clone(), vec![Observation { text }]);
        }
        let digest = self.digest_result(basis, &units);
        Ok(AnalysisResult {
            digest,
            partial: false,
            observations: ObservationSet {
                provider_kind: ProviderKind::CogniCode,
                units,
                restart_observed: false,
            },
        })
    }
}

fn protocol_major(result: Option<&serde_json::Value>) -> Result<u32, CodeIntelligencePortError> {
    let v = result.ok_or(CodeIntelligencePortError::InvalidPayload)?;
    let pv = v
        .get("protocolVersion")
        .and_then(|p| p.as_str())
        .ok_or(CodeIntelligencePortError::InvalidPayload)?;
    let year: String = pv.chars().take_while(|c| c.is_ascii_digit()).collect();
    year.parse::<u32>()
        .map_err(|_| CodeIntelligencePortError::InvalidPayload)
}
