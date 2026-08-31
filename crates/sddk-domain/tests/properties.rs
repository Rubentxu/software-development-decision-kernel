//! Property tests for identity normalization and stable hashing.

use proptest::prelude::*;
use sddk_domain::{
    normalize_remote_url, stable_fallback_project_id, stable_project_id, stable_workspace_id,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn remote_normalization_is_idempotent(url in "[a-zA-Z0-9:/._@-]{0,64}") {
        if let Ok(first) = normalize_remote_url(&url) {
            let second = normalize_remote_url(&first).expect("normalized form is normalizable");
            prop_assert_eq!(first, second);
        }
    }

    #[test]
    fn stable_project_id_is_deterministic(remote in "[a-z0-9.-]{1,32}", scope in "[a-z0-9._/-]{1,32}") {
        let first = stable_project_id(&remote, &scope);
        let second = stable_project_id(&remote, &scope);
        prop_assert_eq!(first.clone(), second);
        prop_assert!(first.starts_with("p-"));
    }

    #[test]
    fn distinct_scopes_do_not_collide(remote in "[a-z0-9.-]{1,16}", scope_a in "[a-z0-9]{1,8}", scope_b in "[a-z0-9]{1,8}") {
        prop_assume!(scope_a != scope_b);
        prop_assert_ne!(stable_project_id(&remote, &scope_a), stable_project_id(&remote, &scope_b));
    }

    #[test]
    fn workspace_id_frames_project_and_path(project in "p-[a-f0-9]{16}", path in "[a-z0-9/._-]{1,48}") {
        let project_id = sddk_domain::ProjectId::new(&project).expect("project id is valid");
        let first = stable_workspace_id(&project_id, &path);
        let second = stable_workspace_id(&project_id, &path);
        prop_assert_eq!(first.clone(), second);
        prop_assert!(first.starts_with("w-"));
    }

    #[test]
    fn fallback_id_is_stable_and_distinct(seed_a in "[0-9a-f-]{8,36}", seed_b in "[0-9a-f-]{8,36}", scope in "[a-z0-9]{1,8}") {
        prop_assume!(seed_a != seed_b);
        prop_assert_eq!(
            stable_fallback_project_id(&seed_a, &scope),
            stable_fallback_project_id(&seed_a, &scope)
        );
        prop_assert_ne!(
            stable_fallback_project_id(&seed_a, &scope),
            stable_fallback_project_id(&seed_b, &scope)
        );
    }
}
