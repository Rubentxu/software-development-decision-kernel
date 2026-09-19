//! Production MCP stdio adapter for the `RuntimeEvidencePort` (A7).
//!
//! Cycle: `p-63676b11dc0ef88f/aiw-s5-chronos-runtime` (AIW-S5).
//!
//! Spawns the real `chronos-mcp` binary over stdio (JSON-RPC line
//! protocol; tracing logs on stderr are consumed, not interpreted).
//! Provider types never cross the boundary: `tools/call` responses
//! are parsed strictly from `content[0].text` into SDDK ADTs; any
//! malformed payload fails closed (`InvalidPayload`).
//!
//! Provider facts (S5 DISCOVERY, observed 2026-09-19):
//! - protocol `2025-03-26`; `tools/call` results are JSON-in-text.
//! - `probe_start {program, args?, cwd?, trace_syscalls?}` returns
//!   `session_id` (also required by drain/stop/summary).
//! - `get_execution_summary` returns typed counts + duration_ns.

#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use crate::code_intelligence_port::DigestSha256;
use crate::evidence_ref::{EvidenceKind, EvidenceRef};
use crate::observation::types::{
    ObservationBasis, ObservationOrigin, ObservationStance, ObservationSubject, SoftwareObservation,
};
use crate::runtime_evidence_port::{
    ObservationSet, RuntimeCapabilitySnapshot, RuntimeCaptureRequest, RuntimeCaptureResult,
    RuntimeEvidencePort, RuntimePortError,
};

enum Reply {
    Message(serde_json::Value),
    Eof,
}

/// One spawned `chronos-mcp` process.
pub struct ChronosMcpAdapter {
    child: Mutex<Child>,
    send: Mutex<std::process::ChildStdin>,
    replies: Receiver<Reply>,
    next_id: Mutex<u64>,
    server_version: String,
}

impl ChronosMcpAdapter {
    /// Spawn `chronos-mcp` and run the MCP initialize handshake.
    /// Fails closed on spawn/handshake failure or protocol-major
    /// mismatch.
    pub fn spawn(
        binary: &str,
        store_path: Option<&str>,
        timeout: Duration,
    ) -> Result<Self, RuntimePortError> {
        let mut cmd = Command::new(binary);
        if let Some(store) = store_path {
            cmd.env("CHRONOS_STORE_PATH", store);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| RuntimePortError::Unavailable)?;

        let stdin = child.stdin.take().ok_or(RuntimePortError::Unavailable)?;
        let stdout = child.stdout.take().ok_or(RuntimePortError::Unavailable)?;
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
                        // Non-JSON stdout lines are protocol violations;
                        // stay tolerant and skip (same policy as S1).
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

        let mut adapter = Self {
            child: Mutex::new(child),
            send: Mutex::new(stdin),
            replies: rx,
            next_id: Mutex::new(1),
            server_version: String::new(),
        };

        // initialize handshake
        let id = adapter.take_id();
        adapter
            .send_json(serde_json::json!({
                "jsonrpc": "2.0", "id": id, "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {"name": "sddk-runtime-port", "version": "0.1.0"}
                }
            }))
            .map_err(|_| RuntimePortError::Unavailable)?;
        let reply = adapter
            .recv_reply(id, timeout)
            .ok_or(RuntimePortError::Unavailable)?;
        let result = reply.get("result").ok_or(RuntimePortError::Unavailable)?;
        let major = result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .and_then(|s| s.split('-').next())
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or(RuntimePortError::InvalidPayload("protocol version"))?;
        if major != 2025 {
            return Err(RuntimePortError::InvalidPayload("protocol major mismatch"));
        }
        adapter.server_version = result
            .get("serverInfo")
            .and_then(|s| s.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_owned();

        // initialized notification (no id, no reply expected)
        adapter
            .send_json(serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .map_err(|_| RuntimePortError::Unavailable)?;

        Ok(adapter)
    }

    fn take_id(&self) -> u64 {
        let mut guard = self.next_id.lock().expect("poisoned id lock");
        let id = *guard;
        *guard += 1;
        id
    }

    fn send_json(&self, v: serde_json::Value) -> Result<(), std::io::Error> {
        let mut stdin = self.send.lock().expect("poisoned stdin lock");
        stdin.write_all(v.to_string().as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
    }

    fn recv_reply(&self, id: u64, timeout: Duration) -> Option<serde_json::Value> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let now = std::time::Instant::now();
            if now >= deadline {
                return None;
            }
            match self.replies.recv_timeout(deadline - now) {
                Ok(Reply::Message(v)) => {
                    if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
                        return Some(v);
                    }
                }
                Ok(Reply::Eof) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return None;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return None,
            }
        }
    }

    /// One tools/call round-trip returning the provider's JSON result
    /// (the `content[0].text` payload re-parsed as JSON). If the
    /// provider marks `isError`, the message text becomes a
    /// `ProviderError`.
    fn call_tool(
        &self,
        tool: &str,
        args: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, RuntimePortError> {
        let id = self.take_id();
        self.send_json(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": tool, "arguments": args}
        }))
        .map_err(|_| RuntimePortError::Unavailable)?;
        let reply = self
            .recv_reply(id, timeout)
            .ok_or(RuntimePortError::Unavailable)?;
        if let Some(err) = reply.get("error") {
            return Err(RuntimePortError::ProviderError(err.to_string()));
        }
        let result = reply
            .get("result")
            .ok_or(RuntimePortError::InvalidPayload("missing result"))?;
        if result.get("isError").and_then(|b| b.as_bool()) == Some(true) {
            let msg = result
                .get("content")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("provider error");
            return Err(RuntimePortError::ProviderError(msg.to_owned()));
        }
        let text = result
            .get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .ok_or(RuntimePortError::InvalidPayload("content[0].text missing"))?;
        serde_json::from_str(text).map_err(|_| RuntimePortError::InvalidPayload("payload JSON"))
    }

    pub fn server_version(&self) -> &str {
        &self.server_version
    }
}

impl Drop for ChronosMcpAdapter {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl RuntimeEvidencePort for ChronosMcpAdapter {
    fn capabilities(&self) -> RuntimeCapabilitySnapshot {
        RuntimeCapabilitySnapshot {
            provider_name: "chronos-mcp".to_owned(),
            provider_version: self.server_version.clone(),
            protocol_major: 2025,
            tools: vec![
                "probe_start".to_owned(),
                "probe_drain".to_owned(),
                "probe_stop".to_owned(),
                "get_execution_summary".to_owned(),
            ],
        }
    }

    fn capture(
        &self,
        req: &RuntimeCaptureRequest,
    ) -> Result<RuntimeCaptureResult, RuntimePortError> {
        let timeout = Duration::from_millis(req.timeout_ms.max(1_000));

        // 1. probe_start
        let mut args = serde_json::json!({"program": req.program, "trace_syscalls": true});
        if !req.args.is_empty() {
            args["args"] = serde_json::json!(req.args);
        }
        if let Some(cwd) = &req.cwd {
            args["cwd"] = serde_json::json!(cwd);
        }
        let started = self.call_tool("probe_start", args, timeout)?;
        let session_id = started
            .get("session_id")
            .and_then(|s| s.as_str())
            .ok_or(RuntimePortError::InvalidPayload("session_id missing"))?
            .to_owned();

        // 2. drain (best effort while the target runs)
        let _ = self.call_tool(
            "probe_drain",
            serde_json::json!({"session_id": session_id}),
            timeout,
        );

        // 3. probe_stop (finalize; safe even if target already exited)
        let _ = self.call_tool(
            "probe_stop",
            serde_json::json!({"session_id": session_id}),
            timeout,
        );

        // 4. get_execution_summary (authoritative typed summary)
        let summary = self.call_tool(
            "get_execution_summary",
            serde_json::json!({"session_id": session_id}),
            timeout,
        )?;

        let total_events = summary
            .get("total_events")
            .and_then(|v| v.as_u64())
            .ok_or(RuntimePortError::InvalidPayload("total_events"))?;
        let duration_ns = summary.get("duration_ns").and_then(|v| v.as_u64());
        let thread_count = summary
            .get("thread_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let mut event_counts = std::collections::BTreeMap::new();
        if let Some(pairs) = summary
            .get("event_counts_by_type")
            .and_then(|v| v.as_array())
        {
            for pair in pairs {
                if let (Some(k), Some(v)) = (
                    pair.get(0).and_then(|x| x.as_str()),
                    pair.get(1).and_then(|x| x.as_u64()),
                ) {
                    event_counts.insert(k.to_owned(), v);
                }
            }
        }
        // The summary does not carry the target's exit status; do NOT
        // infer success. It stays None unless a later provider version
        // exposes it.
        let exit_status: Option<i32> = None;

        // Canonical digest material (same policy as S1: deterministic
        // field ordering, provider strings hashed, no unhashed pass-through).
        let mut material = String::new();
        material.push_str("sddk.runtime.capture.v1|");
        material.push_str(&session_id);
        material.push('|');
        material.push_str(&total_events.to_string());
        material.push('|');
        material.push_str(&duration_ns.map(|d| d.to_string()).unwrap_or_default());
        material.push('|');
        for (k, v) in &event_counts {
            material.push_str(&format!("{k}={v};"));
        }
        material.push('|');
        material.push_str(&thread_count.to_string());
        let digest = DigestSha256::of(material.as_bytes());

        // 5. Typed observation: the capture itself, subject =
        //    `runtime:<program>`, stance Affirms, evidence locator
        //    pointing at the session. Exit status unknown → no claim
        //    about success here; the claim layer (CLI) decides what
        //    "completed" means against these counts.
        let subject_tag = format!("runtime:{}", req.program);
        let subject = ObservationSubject::Unit(crate::architecture_graph::SoftwareUnitRef::new(
            subject_tag.clone(),
        ));
        let evidence = EvidenceRef::new(
            EvidenceKind::Adhoc,
            format!("chronos-mcp://session/{session_id}#events={total_events}"),
        );
        let basis = ObservationBasis::for_provider_result("aiw-s5-capture", &digest.to_string());
        let mut observations = ObservationSet::new();
        observations.push(SoftwareObservation::declare(
            subject,
            ObservationStance::Affirms,
            evidence,
            ObservationOrigin::RuntimeProvider,
            basis,
            None,
            "chronos-mcp",
        ));

        Ok(RuntimeCaptureResult {
            session_id,
            duration_ns,
            total_events,
            event_counts,
            thread_count,
            exit_status,
            digest,
            observations,
        })
    }
}
