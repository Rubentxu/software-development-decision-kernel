// C3i objetivo 5 (roadmap): una única identidad project/workspace/cycle
// coherente del bootstrap.
//
// Cobertura previa (NO duplicada aquí):
// - INC-DEBT-028 (cli.rs::remote_less_workspace_keeps_one_identity_across_commands):
//   project resolve / adopt status / apply / plan acuerdan el MISMO project_id
//   sin remote y a través de comandos.
// - apply_is_strict_about_identity_after_refresh: drift de identidad crea
//   receipt nuevo y no toca el original.
//
// Hueco que este pin cierra (observado en session-37/38): la estabilidad de la
// identidad COMPLETA (project_id + workspace_id + paths derivados) entre
// reinicios de proceso (planes rederivados desde cero) y a TRAVÉS del verbo
// refresh, y que la identidad es INDEPENDIENTE del runtime metadata
// (runtime_version, timestamp, actor) que sí converge.
use sddk_engine::{
    AdoptionPlanInput, AdoptionStatusKind, XdgEnvironment, apply_adoption, plan_adoption,
    read_adoption_receipt, refresh_adoption,
};
use std::fs;
use tempfile::tempdir;

fn input_for(
    home: &std::path::Path,
    root: &std::path::Path,
    display: &str,
    runtime_version: &str,
    timestamp: &str,
    actor: &str,
) -> AdoptionPlanInput {
    AdoptionPlanInput {
        remote_url: Some("https://example.com/acme/repo.git".into()),
        scope: ".".into(),
        fallback_seed: None,
        canonical_workspace_path: root.to_path_buf(),
        display_name: display.into(),
        xdg: XdgEnvironment {
            home: Some(home.to_path_buf()),
            data_home: Some(home.join("data")),
            state_home: Some(home.join("state")),
            cache_home: Some(home.join("cache")),
            ..XdgEnvironment::default()
        },
        sddk_version: "3.6".into(),
        runtime_version: runtime_version.into(),
        timestamp: timestamp.into(),
        actor: actor.into(),
    }
}

// durability-required: two independent plan_adoption calls (simulated process
// restarts) must derive the SAME project_id, workspace_id, ledger path and
// receipt path; apply + refresh must not move any of them.
#[test]
fn bootstrap_identity_is_unique_and_stable_across_restarts_and_refresh() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("repo");
    fs::create_dir_all(&root).unwrap();

    // "Process restart 1": plan derived from scratch.
    let plan1 = plan_adoption(input_for(
        directory.path(),
        &root,
        "repo",
        "2.2.32",
        "2026-09-29T10:00:00Z",
        "actor-a",
    ))
    .unwrap();
    let mut ledger = sddk_storage::Storage::open(&plan1.paths.ledger).unwrap();
    apply_adoption(&plan1, &mut ledger).unwrap();

    // "Process restart 2..4": re-derive the plan from scratch each time with
    // DIFFERENT runtime metadata (new binary version, new timestamp, new
    // actor). Identity components must be identical; only runtime metadata
    // may differ.
    for (i, (rv, ts, actor)) in [
        ("2.2.33", "2026-09-29T11:00:00Z", "actor-b"),
        ("2.2.34", "2026-09-29T12:00:00Z", "actor-c"),
        ("2.2.35", "2026-09-29T13:00:00Z", "actor-d"),
    ]
    .into_iter()
    .enumerate()
    {
        let plan_n =
            plan_adoption(input_for(directory.path(), &root, "repo", rv, ts, actor)).unwrap();
        assert_eq!(
            plan_n.identity.project_id.as_str(),
            plan1.identity.project_id.as_str(),
            "restart {i}: project_id debe ser unico y estable (runtime metadata no participa)"
        );
        assert_eq!(
            plan_n.workspace_id, plan1.workspace_id,
            "restart {i}: workspace_id derivado debe ser estable"
        );
        assert_eq!(
            plan_n.paths.ledger, plan1.paths.ledger,
            "restart {i}: el ledger debe ser el mismo archivo"
        );
        assert_eq!(
            plan_n.paths.receipt, plan1.paths.receipt,
            "restart {i}: el recibo debe ser el mismo archivo"
        );

        // apply con runtime metadata distinta CONVERGE: reescribe runtime
        // metadata (contrato obj 2) pero JAMAS identidad. La byte-estabilidad
        // solo aplica con fingerprint identico y ya esta pineada en
        // apply_on_converged_adoption_is_byte_stable_across_repeats.
        let mut ledger_n = sddk_storage::Storage::open(&plan_n.paths.ledger).unwrap();
        let st = apply_adoption(&plan_n, &mut ledger_n).unwrap();
        assert_eq!(st.status, AdoptionStatusKind::Complete);
        let after_apply = read_adoption_receipt(&plan_n.paths.receipt).unwrap();
        assert_eq!(
            after_apply.project_id,
            plan1.identity.project_id.as_str(),
            "restart {i}: re-apply con runtime distinto NO mueve project_id"
        );
        assert_eq!(
            after_apply.workspace_id, plan1.workspace_id,
            "restart {i}: re-apply con runtime distinto NO mueve workspace_id"
        );
        let refreshed = refresh_adoption(&plan_n, &mut ledger_n).unwrap();
        assert_eq!(refreshed.status, AdoptionStatusKind::Complete);
        let on_disk = refreshed.receipt.unwrap();
        assert_eq!(
            on_disk.project_id,
            plan1.identity.project_id.as_str(),
            "restart {i}: refresh NO mueve project_id"
        );
        assert_eq!(
            on_disk.workspace_id, plan1.workspace_id,
            "restart {i}: refresh NO mueve workspace_id"
        );
        assert_eq!(on_disk.paths.ledger, plan1.receipt.paths.ledger);
    }
}
