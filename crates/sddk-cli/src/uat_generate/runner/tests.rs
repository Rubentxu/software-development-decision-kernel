//! Tests for the runner pipeline.

use std::sync::{Arc, Mutex};

use crate::uat_common::io::{ApprovalDecision, ApprovalIo, ApprovalVerdict, UatPlanSummary};

use super::{PipelineConfig, PipelineError, StageOutput, render_pipeline_output, run_pipeline};

/// Scripted approval that always approves.
struct FakeApprove;
impl ApprovalIo for FakeApprove {
    fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
        Ok(ApprovalDecision::new(
            ApprovalVerdict::Approve,
            "T-test".to_string(),
            "Test User".to_string(),
        ))
    }
    fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Scripted approval that always rejects.
struct FakeReject;
impl ApprovalIo for FakeReject {
    fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
        Ok(ApprovalDecision::new(
            ApprovalVerdict::Reject,
            "T-test".to_string(),
            "Test User".to_string(),
        ))
    }
    fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Scripted approval that always requests edit.
struct FakeEdit;
impl ApprovalIo for FakeEdit {
    fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
        Ok(ApprovalDecision::new(
            ApprovalVerdict::Edit,
            "T-test".to_string(),
            "Test User".to_string(),
        ))
    }
    fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn pipeline_empty_source_returns_validation_error() {
    let td = tempfile::TempDir::new().unwrap();

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: None,
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: false,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: None,
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, PipelineError::ValidationFailed(_)));
    assert!(format!("{:?}", err).contains("RequirementsRequired"));
}

#[test]
fn pipeline_reject_approval_returns_approval_rejected() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login Feature\n- User can login with email and password\n- System returns JSON response with status 200\n",
    )
    .unwrap();

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: true,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: Some(Box::new(FakeReject)),
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(result.is_err());
    let e = result.unwrap_err();
    assert!(matches!(e, PipelineError::ApprovalRejected));

    let output_path = td.path().join("uat-plan.yaml");
    assert!(
        !output_path.exists(),
        "output file should not exist after rejection"
    );
}

#[test]
fn pipeline_approve_creates_output_with_approval() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login Feature\n- User can login with email and password\n",
    )
    .unwrap();

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: true,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: Some(Box::new(FakeApprove)),
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(result.is_ok(), "pipeline should succeed: {:?}", result);
    let _stages = result.unwrap();

    let output_path = td.path().join("uat-plan.yaml");
    assert!(
        output_path.exists(),
        "output file should exist after approval"
    );

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains("approval"),
        "plan should contain approval record"
    );
}

#[test]
fn pipeline_auto_skips_approval() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login Feature\n- User can login with email and password\n",
    )
    .unwrap();

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: false,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: None,
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(result.is_ok(), "auto mode should succeed: {:?}", result);
    let stages = result.unwrap();

    let approval_stage = stages.iter().find(|s| s.stage == "approval");
    assert!(
        approval_stage.is_some_and(|s| s.tag == "auto_skip"),
        "approval stage should be auto_skip in non-interactive mode"
    );
}

#[test]
fn pipeline_edit_returns_approval_edit_requested() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login Feature\n- User can login with email and password\n",
    )
    .unwrap();

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: true,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: Some(Box::new(FakeEdit)),
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(result.is_err());
    let e = result.unwrap_err();
    assert!(matches!(e, PipelineError::ApprovalEditRequested));

    let output_path = td.path().join("uat-plan.yaml");
    assert!(
        !output_path.exists(),
        "output file should not exist after edit request"
    );
}

#[test]
fn pipeline_atomic_no_partial_on_quality_failure() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login\n- User can login\n",
    )
    .unwrap();

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: false,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: None,
        force_quality_failure: true,
    };

    let result = run_pipeline(config);
    assert!(
        matches!(result, Err(PipelineError::QualityFailed(_))),
        "Expected QualityFailed, got: {:?}",
        result
    );

    let output_path = td.path().join("uat-plan.yaml");
    assert!(
        !output_path.exists(),
        "no output file should exist on pipeline failure"
    );

    let tmp_files: Vec<_> = std::fs::read_dir(td.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .collect();
    assert!(
        tmp_files.is_empty(),
        "no .tmp-* files should exist after failure"
    );
}

#[test]
fn pipeline_approve_calls_io_record_before_persistence() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login\n- User can login\n",
    )
    .unwrap();

    let record_called = Arc::new(Mutex::new(false));
    let record_called_clone = record_called.clone();

    struct FakeApproveWithRecord {
        called: Arc<Mutex<bool>>,
    }
    impl ApprovalIo for FakeApproveWithRecord {
        fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
            Ok(ApprovalDecision::new(
                ApprovalVerdict::Approve,
                "T-test".to_string(),
                "Test User".to_string(),
            ))
        }
        fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
            *self.called.lock().unwrap() = true;
            Ok(())
        }
    }

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: true,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: Some(Box::new(FakeApproveWithRecord {
            called: record_called_clone,
        })),
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "pipeline should succeed with FakeApprove: {:?}",
        result
    );

    assert!(
        *record_called.lock().unwrap(),
        "io.record must be called after approval before persistence"
    );

    let output_path = td.path().join("uat-plan.yaml");
    assert!(
        output_path.exists(),
        "output should exist after successful pipeline"
    );
}

#[test]
fn render_pipeline_output_includes_tags_and_path() {
    use std::path::PathBuf;

    let stages = vec![
        StageOutput {
            stage: "discover",
            path: PathBuf::from("N/A"),
            tag: "skipped".to_string(),
            message: "discover: skipped".to_string(),
        },
        StageOutput {
            stage: "plan",
            path: PathBuf::from("N/A"),
            tag: "planned".to_string(),
            message: "plan: 1 features".to_string(),
        },
        StageOutput {
            stage: "write",
            path: PathBuf::from("/tmp/uat-plan-v1.0.0.yaml"),
            tag: "written".to_string(),
            message: "written: /tmp/uat-plan-v1.0.0.yaml".to_string(),
        },
    ];
    let final_path = PathBuf::from("/tmp/uat-plan-v1.0.0.yaml");

    let output = render_pipeline_output(&stages, &final_path);

    assert!(
        output.contains("[discover]"),
        "output must include discover stage tag"
    );
    assert!(
        output.contains("[plan]"),
        "output must include plan stage tag"
    );
    assert!(
        output.contains("[write]"),
        "output must include write stage tag"
    );
    assert!(output.contains("skipped"), "output must include tag values");
    assert!(output.contains("planned"), "output must include tag values");
    assert!(output.contains("written"), "output must include tag values");
    assert!(
        output.contains("/tmp/uat-plan-v1.0.0.yaml"),
        "output must include final path"
    );
    assert!(
        output.contains("Pipeline complete"),
        "output must include Pipeline complete marker"
    );
}

#[test]
fn pipeline_auto_mode_approval_absent() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login\n- User can login with email and password\n",
    )
    .unwrap();

    let output_path = td.path().join("uat-plan.yaml");
    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: false,
        output: Some(output_path.clone()),
        approval_io: None,
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(result.is_ok(), "auto mode should succeed: {:?}", result);

    assert!(output_path.exists(), "output should exist in auto mode");

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        !content.contains("approval:"),
        "auto mode should not include approval in output"
    );
}

#[test]
fn pipeline_atomic_no_output_on_record_failure() {
    let td = tempfile::TempDir::new().unwrap();
    let req_dir = td.path();
    std::fs::write(
        req_dir.join("req.md"),
        "# Requirements\n\n## Login\n- User can login\n",
    )
    .unwrap();

    let record_called = Arc::new(Mutex::new(false));
    let record_called_clone = record_called.clone();

    struct FakeApproveWithRecordError {
        called: Arc<Mutex<bool>>,
    }
    impl ApprovalIo for FakeApproveWithRecordError {
        fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
            Ok(ApprovalDecision::new(
                ApprovalVerdict::Approve,
                "T-test".to_string(),
                "Test User".to_string(),
            ))
        }
        fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
            *self.called.lock().unwrap() = true;
            Err(anyhow::anyhow!("simulated record failure"))
        }
    }

    let config = PipelineConfig {
        release: "v1.0.0".to_string(),
        requirements: Some(req_dir.to_path_buf()),
        changelog: None,
        last_plan: None,
        discover: false,
        app_url: None,
        interactive: true,
        output: Some(td.path().join("uat-plan.yaml")),
        approval_io: Some(Box::new(FakeApproveWithRecordError {
            called: record_called_clone,
        })),
        force_quality_failure: false,
    };

    let result = run_pipeline(config);
    assert!(
        result.is_err(),
        "Expected pipeline to fail on record error, got: {:?}",
        result
    );

    assert!(
        *record_called.lock().unwrap(),
        "io.record must have been called"
    );

    let output_path = td.path().join("uat-plan.yaml");
    assert!(
        !output_path.exists(),
        "no output file should exist when record fails"
    );
}
