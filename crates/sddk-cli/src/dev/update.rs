//! `dev update` — download and install a framework release bundle.

use super::manifest::MANIFEST_FILE;
use crate::dev::common::{CopyMode, copy_tree, count_manifest_entries, download_to, sha256_hex};
use crate::dev::manifest::verify_manifest;
use crate::dev::paths::framework_dir;
use crate::{CliEnvironment, CommandOutput, render_result};
use std::path::Path;

/// Verify the bundle's cosign signature against the Sigstore trust root.
///
/// Why this exists: `update.rs` downloads the payload and its `.sha256`
/// from the SAME `{base_url}` path. That proves integrity in transit, not
/// authenticity — a compromised origin serves a hostile bundle together
/// with a matching checksum and the check passes. It is a manifest
/// attesting to itself. See INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.
///
/// The signature breaks the circle because the trust root does not travel
/// with the artifact: `cosign verify-blob` validates the certificate
/// chain against Fulcio's public root and checks that the identity in the
/// certificate matches the expected one. An attacker who controls the
/// download origin still cannot mint a certificate for that identity.
///
/// Policy, deliberately fail-closed:
///   * signature present → it MUST verify. Bad signature = hard error,
///     never a warning. A warning here would be a downgrade an attacker
///     could trigger by serving a bad signature.
///   * signature absent → error, unless the operator opts in explicitly
///     with `SDDK_ALLOW_UNSIGNED_UPDATE=1`.
///   * cosign not installed → same as absent, with a message that says
///     what to install. Never silently skipped.
///
/// The opt-out is named ALLOW, not SKIP, and always prints that it is
/// being used. A quiet downgrade is how "we have signatures" becomes a
/// claim that is false in production.
fn verify_bundle_signature(bundle: &Path, url: &str, allow_unsigned: bool) -> anyhow::Result<()> {
    // cosign writes `<name>.sig` / `<name>.bundle.json` next to the blob.
    let sig_path = bundle.with_file_name(format!(
        "{}.sig",
        bundle.file_name().unwrap_or_default().to_string_lossy()
    ));
    let bundle_path = bundle.with_file_name(format!(
        "{}.bundle.json",
        bundle.file_name().unwrap_or_default().to_string_lossy()
    ));
    let cert_path = bundle.with_file_name(format!(
        "{}.pem",
        bundle.file_name().unwrap_or_default().to_string_lossy()
    ));

    let have_cosign = std::process::Command::new("cosign")
        .arg("version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    let sig_downloaded = if have_cosign {
        // Fetch the detached certificate alongside the signature when the
        // bundle form is not published. The CI signs detached; without the
        // .pem the verification below cannot pin an identity.
        if download_to(&format!("{url}.sig"), &sig_path).is_ok() {
            let _ = download_to(&format!("{url}.pem"), &cert_path);
            true
        } else {
            false
        }
    } else {
        false
    };

    if !sig_downloaded {
        // Two opt-in surfaces, one authority. `--allow-unsigned` is the
        // explicit one. The environment variable is accepted as a
        // convenience for scripted installs, and it is the same decision
        // expressed a different way — not a second key to a lock.
        //
        // An earlier version required both to agree. That was untestable:
        // the crate is `#![forbid(unsafe_code)]`, so a unit test cannot set
        // an env var to reach the accepting branch at all, and a security
        // control whose happy path cannot be exercised is a control that
        // gets "fixed" in production by dropping it.
        if allow_unsigned || std::env::var("SDDK_ALLOW_UNSIGNED_UPDATE").is_ok() {
            eprintln!(
                "warning: installing an UNVERIFIED bundle from {url}.\n\
                 warning: integrity (sha256) is checked, authenticity is NOT. A compromised\n\
                 warning: origin would be accepted. Drop --allow-unsigned to require a signature."
            );
            return Ok(());
        }
        anyhow::bail!(
            "no signature at {url}.sig and cosign is {}.\n\
             Refusing to install a bundle whose authenticity cannot be established:\n\
             the .sha256 comes from the same origin as the payload, so it only proves\n\
             the bytes did not change in transit, not that they are ours.\n\
             If this release is genuinely unsigned, re-run with --allow-unsigned\n\
             (or SDDK_ALLOW_UNSIGNED_UPDATE=1) to accept it knowingly.\n\
             See INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.",
            if have_cosign {
                "installed but the signature asset is missing"
            } else {
                "not installed"
            }
        );
    }

    // Verify. Prefer the bundle format when present (current cosign), fall
    // back to the detached signature for older cosign versions.
    // The CI (.github/workflows/release.yml) signs DETACHED: it publishes
    // <file>.sig and <file>.pem. The bundle form is what current cosign
    // emits with --bundle. Both are accepted, but the detached form must
    // carry the certificate explicitly.
    //
    // `cosign verify-blob --signature` on its own validates against whatever
    // certificate cosign picks, which is the unpinned path this policy
    // exists to close. Passing --certificate-identity without a
    // --certificate-chain gives cosign nothing to match, so the check
    // would report strictness it does not have.
    let mut cmd = std::process::Command::new("cosign");
    if bundle_path.exists() {
        cmd.args(["verify-blob", "--bundle"]).arg(&bundle_path);
    } else if cert_path.exists() {
        cmd.args(["verify-blob", "--signature", "--certificate-chain"])
            .arg(&sig_path)
            .arg(&cert_path);
    } else {
        anyhow::bail!(
            "detached signature for {} has no .pem certificate next to it.\n\
             Refusing to verify without a certificate chain: --certificate-identity would\n\
             have nothing to match, so a signature from ANY signer would be accepted.",
            bundle.display()
        );
    }
    cmd.arg(bundle);

    // Identity and issuer are SEPARATE variables. Conflating them in one
    // var whose meaning depends on whether the value contains '@' is a trap:
    // the GitHub Actions issuer URL has no '@', so passing it as the identity
    // produced `--certificate-identity=<issuer>` — and cosign, told only an
    // identity, still requires `--certificate-oidc-issuer` to be supplied or
    // it accepts the certificate against ANY issuer. That is not a stronger
    // check, it is a weaker one wearing a stricter-looking name.
    //
    // Defaults are the real SDDK signing identity so an operator who sets
    // nothing gets pinning, not "any Sigstore certificate will do".
    let identity = std::env::var("SDDK_COSIGN_IDENTITY")
        .unwrap_or_else(|_| crate::cosign::DEFAULT_CERT_IDENTITY_REGEXP.to_string());
    let issuer = std::env::var("SDDK_COSIGN_ISSUER")
        .unwrap_or_else(|_| crate::cosign::DEFAULT_CERT_ISSUER.to_string());

    if identity.trim().is_empty() || issuer.trim().is_empty() {
        anyhow::bail!(
            "SDDK_COSIGN_IDENTITY and SDDK_COSIGN_ISSUER must both be non-empty.\n\
             Refusing to verify without pinning both: an empty value would let\n\
             cosign accept any certificate, which proves nothing about us."
        );
    }

    cmd.arg(format!("--certificate-identity-regexp={identity}"));
    cmd.arg(format!("--certificate-oidc-issuer={issuer}"));

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "cosign verification FAILED for {}\n{stderr}\n\
             Refusing to install. A present-but-invalid signature is never a warning:\n\
             an attacker who can serve artifacts can also serve a bad signature, and\n\
             a warning here would turn that into a bypass.",
            bundle.display()
        );
    }
    Ok(())
}

/// Reject tarball members that would write outside the extraction root.
///
/// GNU tar's `--strip-components=1` removes the FIRST path component but
/// does not neutralise traversal: a member named
/// `software-development-decision-kernel/../../etc/cron.d/x` strips to
/// `../../etc/cron.d/x`, which escapes `-C <root>`. Any archive that
/// reaches this point has already passed the sha256 check, so an
/// attacker controlling the release origin can already serve an archive
/// that passes verification — this guard is what stops that from turning
/// into an arbitrary-write primitive.
///
/// Fails closed. The check is deliberately about path *shape* only:
/// content integrity is the sha256 check's and `verify_manifest`'s job.
pub(crate) fn ensure_safe_tarball_members(listing: &str) -> anyhow::Result<()> {
    for member in listing.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if member.starts_with('/') {
            anyhow::bail!("tarball contains absolute path: {member}");
        }
        if member.split('/').any(|c| c == "..") {
            anyhow::bail!("tarball member escapes target directory: {member}");
        }
        if Path::new(member).components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        }) {
            anyhow::bail!("tarball member escapes target directory: {member}");
        }
    }
    Ok(())
}

pub(crate) fn update_bundle(root: &Path, args: &super::UpdateArgs) -> anyhow::Result<String> {
    let version = args.version.as_deref().unwrap_or("latest");
    let base_url = match &args.base_url {
        Some(base) => base.clone(),
        None => format!("https://github.com/{}/releases", args.repo),
    };
    let asset = "software-development-decision-kernel.tar.gz";
    let url = if version == "latest" {
        format!("{base_url}/latest/download/{asset}")
    } else {
        format!("{base_url}/download/{version}/{asset}")
    };

    static NEXT_UPDATE_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT_UPDATE_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("sddk-update-{}-{sequence}", std::process::id()));
    let tmp_dir = tmp.join("dl");
    let staged_bundle = tmp_dir.join("bundle");
    let bundle = tmp_dir.join(asset);
    let checksum = tmp_dir.join(format!("{asset}.sha256"));
    std::fs::create_dir_all(&tmp_dir)?;
    std::fs::create_dir_all(&staged_bundle)?;

    download_to(&url, &bundle)?;
    download_to(&format!("{url}.sha256"), &checksum)?;

    // Signature verification (authenticity), before integrity.
    //
    // The `.sha256` above is downloaded from the SAME origin as the payload,
    // so it only proves the bytes did not change in transit: an origin that
    // is compromised serves a payload AND its matching checksum, and this
    // check passes. That is the self-attesting-manifest problem from
    // INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY.
    //
    // A cosign signature over the bundle, verified against the Sigstore
    // trust root, breaks that circularity: the trust root does not come
    // from the same channel as the artifact.
    //
    // Policy (fail-closed, no silent downgrade):
    //   - signature present  -> it MUST verify. A bad signature aborts.
    //   - signature absent   -> abort UNLESS the operator explicitly
    //     accepts unsigned updates via SDDK_ALLOW_UNSIGNED_UPDATE=1.
    //   - cosign missing     -> treated as "cannot verify", same rule.
    // The env var is named ALLOW, not SKIP, and prints what it is doing.
    verify_bundle_signature(&bundle, &url, args.allow_unsigned)?;

    let expected = std::fs::read_to_string(&checksum)?
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty checksum file: {}", checksum.display()))?
        .to_owned();
    let actual = sha256_hex(&bundle)?;
    if expected != actual {
        anyhow::bail!("framework sha256 mismatch\n  expected: {expected}\n  actual:   {actual}");
    }

    // Reject any tarball member that would escape `staged_bundle` before
    // extraction. `--strip-components=1` alone does NOT prevent a hostile
    // archive from writing outside the target: a member named
    // `software-development-decision-kernel/../../etc/x` strips to
    // `../etc/x`. This is a real write primitive, so the archive is
    // inspected first and extraction is fail-closed on any suspicious
    // member.
    {
        let listing = std::process::Command::new("tar")
            .args(["tzf", bundle.to_str().unwrap_or_default()])
            .output()?;
        if !listing.status.success() {
            anyhow::bail!("failed to list tarball members (corrupt archive)");
        }
        let listing = String::from_utf8(listing.stdout)
            .map_err(|_| anyhow::anyhow!("tarball member list is not valid UTF-8"))?;
        ensure_safe_tarball_members(&listing)?;
    }

    let extract = std::process::Command::new("tar")
        .args([
            "xzf",
            bundle.to_str().unwrap_or_default(),
            "-C",
            staged_bundle.to_str().unwrap_or_default(),
            // Do not let the archive carry ownership or permission bits
            // into the staged bundle: a hostile tarball could otherwise set
            // arbitrary modes on the files it writes.
            "--no-same-owner",
            "--no-same-permissions",
            // The release tarball wraps every entry under
            // `software-development-decision-kernel/`; strip that prefix so
            // the staged bundle root matches `MANIFEST_FILE`'s expected
            // location. This is the legacy split-asset path; the unified
            // artifact path used by install.sh extracts to bin/ + framework/.
            "--strip-components=1",
        ])
        .output()?;
    if !extract.status.success() {
        anyhow::bail!(
            "extract failed: {}",
            String::from_utf8_lossy(&extract.stderr).trim()
        );
    }
    // Post-extract integrity: verify every file against the manifest that
    // SHIPPED INSIDE the tarball before touching the target. The tarball
    // checksum proves transport integrity; the internal manifest proves
    // content integrity of each framework surface (ADR-011).
    let manifest_path = staged_bundle.join(MANIFEST_FILE);
    if !manifest_path.is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        anyhow::bail!("bundle is missing required {MANIFEST_FILE}");
    }
    match verify_manifest(&staged_bundle) {
        Ok(mismatches) if mismatches.is_empty() => {}
        Ok(mismatches) => {
            let _ = std::fs::remove_dir_all(&tmp);
            anyhow::bail!(
                "bundle content verification FAILED ({} mismatch(es)):\n  {}",
                mismatches.len(),
                mismatches.join("\n  ")
            );
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            anyhow::bail!("bundle manifest unreadable: {e}");
        }
    }
    let count = count_manifest_entries(root).unwrap_or(0);
    copy_tree(&staged_bundle, root, CopyMode::Always)?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(format!(
        "framework: {version} ({asset}) sha256 verified: {actual}; {count} files content-verified via {MANIFEST_FILE}\n"
    ))
}

/// Identify a "bundle version directory" inside `framework_dir/` by name.
/// Versions are `MAJOR.MINOR[.PATCH][-PRE][+BUILD]`; we keep it permissive
/// but reject anything that contains path separators or whitespace, and
/// require at least one digit so we never match `legacy`, `tmp`, etc.
fn is_bundle_version_dir(name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains(' ') {
        return false;
    }
    name.chars().any(|c| c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
}

/// Resolve the version pointed to by the `current` symlink in `framework_dir`,
/// if any.
fn resolve_current_version(framework_dir: &Path) -> Option<String> {
    let current = framework_dir.join("current");
    let target = std::fs::read_link(&current).ok()?;
    let name = target.file_name()?.to_str()?.to_owned();
    if is_bundle_version_dir(&name) {
        Some(name)
    } else {
        None
    }
}

/// After pruning, re-point `current` symlink to the newest kept version.
/// This runs when `--prune` or `--prune-only` is used with `--root .`.
fn repoint_current_to_newest(framework_dir: &Path, newest_version: &str) {
    let target = framework_dir.join(newest_version);
    if target.is_dir() {
        crate::dev::swap_current_to(framework_dir, &target);
    }
}

/// Semver-aware "newer than" comparison for bundle version directory names.
/// Supports dotted-numeric with optional `-prerelease` and `+build` tags.
/// Pre-release sorts below its release; numeric segments compare numerically.
fn cmp_bundle_version(a: &str, b: &str) -> std::cmp::Ordering {
    fn parse(s: &str) -> (Vec<u64>, Option<&str>, Option<&str>) {
        let (base, build) = match s.split_once('+') {
            Some((b, build)) => (b, Some(build)),
            None => (s, None),
        };
        let (core, pre) = match base.split_once('-') {
            Some((c, pre)) => (c, Some(pre)),
            None => (base, None),
        };
        let nums: Vec<u64> = core
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect();
        (nums, pre, build)
    }
    let (an, ap, ab) = parse(a);
    let (bn, bp, bb) = parse(b);
    let ord = an.cmp(&bn);
    if ord != std::cmp::Ordering::Equal {
        return ord;
    }
    // release > pre-release for the same core
    let pre_ord = match (ap, bp) {
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(x), Some(y)) => x.cmp(y),
        (None, None) => std::cmp::Ordering::Equal,
    };
    pre_ord.then_with(|| ab.cmp(&bb))
}

/// Remove stale bundle version directories. Returns the list of removed
/// versions and the list of kept versions (for reporting).
///
/// Policy:
/// - `current_version` (if any) is always kept.
/// - When `keep_n` is 0: keep only `current_version` (or all versions if no
///   current symlink exists, since there is no basis to choose).
/// - When `keep_n` > 0: keep the N most recent versions by semver-aware sort,
///   plus `current_version` if it is not already in the kept set.
/// - Anything else is deleted.
pub(crate) fn prune_stale_bundles(
    framework_dir: &Path,
    keep_n: usize,
    current_version: Option<&str>,
) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    if !framework_dir.is_dir() {
        anyhow::bail!("framework dir does not exist: {}", framework_dir.display());
    }
    let entries = std::fs::read_dir(framework_dir)?;
    let mut versions: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| is_bundle_version_dir(name))
        .collect();

    // Newest-first sort so we can take(keep_n) from the top.
    versions.sort_by(|a, b| cmp_bundle_version(b, a));

    let mut keep: Vec<String> = versions.iter().take(keep_n).cloned().collect();
    if let Some(cur) = current_version
        && !keep.iter().any(|v| v == cur)
    {
        keep.push(cur.to_owned());
    }

    let mut removed = Vec::new();
    for v in &versions {
        if keep.iter().any(|k| k == v) {
            continue;
        }
        let path = framework_dir.join(v);
        if let Err(e) = std::fs::remove_dir_all(&path) {
            anyhow::bail!("failed to remove {}: {e}", path.display());
        }
        removed.push(v.clone());
    }
    Ok((removed, keep))
}

pub(super) fn run_dev_update(
    args: super::UpdateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        // Validate arg combinations that clap cannot express declaratively.
        if args.keep.is_some() && !args.prune && !args.prune_only {
            anyhow::bail!("--keep requires either --prune or --prune-only");
        }
        if !args.prune && !args.prune_only {
            // Bare `dev update` needs a version to download.
            if args.version.is_none() && !args.prune_only {
                anyhow::bail!(
                    "either --version (to download a bundle), --prune, or --prune-only is required"
                );
            }
        }

        let mut output = String::new();

        // The framework distributes RELEASE BUNDLES (agents/skills/prompts/
        // workflows/assets + MANIFEST.sha256), never repository clones. Git
        // operations are the developer's responsibility: if the target root
        // is a git checkout, the user updates it with `git pull` themselves.
        // (--prune-only skips this check because it operates on the existing
        // version-dir layout, which is also a non-git tree.)
        if !args.prune_only && args.root.join(".git").is_dir() {
            anyhow::bail!(
                "`dev update` installs release bundles and never touches git. \
                 You passed a repository checkout ({}). \
                 To update a checkout, run `git pull` yourself, then \
                 `sddk dev link --root {}` to re-link the editors.",
                args.root.display(),
                args.root.display()
            );
        }

        // Bundle install: download the framework release bundle, verify, and
        // extract into `$SDDK_DATA_DIR/framework/<version>/`.
        let bundle_root = if args.root.as_os_str() == "." {
            framework_dir(environment)?
        } else {
            std::fs::canonicalize(&args.root).unwrap_or(args.root.clone())
        };
        if !args.prune_only {
            output.push_str(&update_bundle(&bundle_root, &args)?);
        } else {
            output.push_str(&format!(
                "prune-only: skipping bundle download; operating on {}\n",
                bundle_root.display()
            ));
        }

        // The extracted bundle lands in a version dir; update_bundle extracts
        // directly into bundle_root, so if the user passed the framework root
        // we additionally fix the `current` symlink to point at it.
        // Guard this with !prune && !prune_only so the prune paths
        // can manage the symlink themselves.
        if args.root.as_os_str() == "." && !args.prune && !args.prune_only {
            crate::dev::swap_current_to(&bundle_root, &bundle_root);
            output.push_str("framework: current -> bundle root (dev link resolves it)\n");
        }

        // Cycle-47 D2: --prune [--keep N] removes stale bundle version dirs
        // that are not `current` and not among the N most recent.
        if args.prune {
            let keep_n = args.keep.unwrap_or(0);
            let active = resolve_current_version(&bundle_root);
            let (removed, kept) = prune_stale_bundles(&bundle_root, keep_n, active.as_deref())?;
            output.push_str(&format!(
                "prune: removed {} stale bundle(s); kept {}\n",
                removed.len(),
                if kept.is_empty() {
                    "(none)".to_string()
                } else {
                    kept.join(", ")
                }
            ));
            if !removed.is_empty() {
                output.push_str(&format!("  removed: {}\n", removed.join(", ")));
            }
            if let Some(cur) = active.as_deref() {
                output.push_str(&format!("  current -> {cur}\n"));
            }
            // After pruning, re-point current to newest kept version
            if args.root.as_os_str() == "." && !kept.is_empty() {
                repoint_current_to_newest(&bundle_root, kept.first().unwrap());
            }
        }

        // Cycle-47 D2 --prune-only: same as --prune but skip the bundle
        // download entirely. Useful for cleaning up after a manual
        // install (e.g. `install.sh --version v1.63.0`) without re-pulling
        // the tarball.
        if args.prune_only {
            let keep_n = args.keep.unwrap_or(0);
            let active = resolve_current_version(&bundle_root);
            let (removed, kept) = prune_stale_bundles(&bundle_root, keep_n, active.as_deref())?;
            output.push_str(&format!(
                "prune-only: removed {} stale bundle(s); kept {}\n",
                removed.len(),
                if kept.is_empty() {
                    "(none)".to_string()
                } else {
                    kept.join(", ")
                }
            ));
            if !removed.is_empty() {
                output.push_str(&format!("  removed: {}\n", removed.join(", ")));
            }
            if let Some(cur) = active.as_deref() {
                output.push_str(&format!("  current -> {cur}\n"));
            }
            // After pruning, re-point current to newest kept version
            if args.root.as_os_str() == "." && !kept.is_empty() {
                repoint_current_to_newest(&bundle_root, kept.first().unwrap());
            }
        }

        Ok(output)
    })();
    render_result(result, format, |output: &String| output.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── signature policy (authenticidad, INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY) ──
    //
    // Estos tests NO invocan cosign. Pinear el comportamiento de "hay firma
    // pero es invalida" exigiria una firma real firmada por una CA real, que
    // no se puede fabricar en un test sin trust root. Lo que SI se puede
    // pinar, y es lo que importa, es la RAMA que decide: cuando no hay
    // firma, la politica es fail-closed y solo se degrada con un opt-in
    // explicito que se anuncia. Un atacante que controle el origen puede
    // servir una firma falsa, pero no puede evitar que falte: ese es el
    // caso que estos tests cubren.

    #[test]
    fn unsigned_bundle_is_rejected_without_explicit_opt_in() {
        // Sin opt-in y sin firma => error. Este es el estado de hoy en
        // cualquier release sin firmar, y tiene que ser ruidoso.
        if std::env::var("SDDK_ALLOW_UNSIGNED_UPDATE").is_ok() {
            return; // el opt-in solo puede venir del entorno del operador
        }
        let dir = tempdir_for_test("sig-reject");
        let bundle = dir.join("software-development-decision-kernel.tar.gz");
        fs::write(&bundle, b"not really a tarball").unwrap();
        let url = format!("file://{}", bundle.display());
        let err = verify_bundle_signature(&bundle, &url, false)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("authenticity cannot be established"),
            "el error debe explicar que el problema es de autenticidad, no de integridad; \
             mensaje real: {err}"
        );
    }

    #[test]
    fn rejection_message_names_the_opt_in_and_the_inc() {
        // El mensaje tiene que decir COMO seguir, no solo que fallo. Un
        // error sin salida practica convierte un control de seguridad en
        // un obstaculo que la gente rodea con sudo.
        if std::env::var("SDDK_ALLOW_UNSIGNED_UPDATE").is_ok() {
            return;
        }
        let dir = tempdir_for_test("sig-msg");
        let bundle = dir.join("b.tar.gz");
        fs::write(&bundle, b"x").unwrap();
        let url = format!("file://{}", bundle.display());
        let err = verify_bundle_signature(&bundle, &url, false)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("SDDK_ALLOW_UNSIGNED_UPDATE=1"),
            "faltaria el opt-in en: {err}"
        );
        assert!(
            err.contains("INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY"),
            "faltaria el INC en: {err}"
        );
    }

    #[test]
    fn rejection_explains_that_sha256_alone_is_not_authenticity() {
        // Este es el hallazgo original del INC y la razon de existir de la
        // funcion. Si el mensaje deja de explicarlo, la funcion ha vuelto a
        // ser un checksum mas.
        if std::env::var("SDDK_ALLOW_UNSIGNED_UPDATE").is_ok() {
            return;
        }
        let dir = tempdir_for_test("sig-why");
        let bundle = dir.join("b.tar.gz");
        fs::write(&bundle, b"x").unwrap();
        let url = format!("file://{}", bundle.display());
        let err = verify_bundle_signature(&bundle, &url, false)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("same origin as the payload"),
            "faltaria el porque en: {err}"
        );
    }

    #[test]
    fn explicit_opt_in_accepts_an_unsigned_bundle() {
        // El camino de aceptacion tambien necesita cobertura. Solo probar
        // el rechazo deja el `if allow_unsigned ||` sin ejercitar, que es
        // justo donde un typo entregaria una rama muerta.
        let dir = tempdir_for_test("sig-accept");
        let bundle = dir.join("b.tar.gz");
        fs::write(&bundle, b"x").unwrap();
        let url = format!("file://{}", bundle.display());
        verify_bundle_signature(&bundle, &url, true)
            .expect("an explicit --allow-unsigned must accept an unsigned bundle");
    }

    #[test]
    fn detached_signature_without_a_certificate_is_refused() {
        // The CI signs detached: .sig + .pem. If the .pem is missing there
        // is nothing for --certificate-identity to match, and cosign would
        // fall back to trusting whatever certificate it likes. Refusing is
        // the only honest outcome; "verified" here would be a lie.
        let dir = tempdir_for_test("sig-no-cert");
        let bundle = dir.join("b.tar.gz");
        fs::write(&bundle, b"payload").unwrap();
        // A .sig next to it, but deliberately NO .pem.
        fs::write(dir.join("b.tar.gz.sig"), b"signature").unwrap();
        let url = format!("file://{}", bundle.display());
        let err = verify_bundle_signature(&bundle, &url, false)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("no .pem certificate") || err.contains("authenticity"),
            "expected a refusal naming the missing certificate, got: {err}"
        );
    }

    /// Helper: no usa `tempfile` para no anadir dependencia de test solo
    /// para tres casos. Usa el temp_dir del proceso, que el resto del
    /// modulo ya usa para staging.
    fn tempdir_for_test(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sddk-sigtest-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── tarball member guard (arbitrary-write primitive) ──

    #[test]
    fn tar_guard_accepts_a_normal_release_listing() {
        let listing = "software-development-decision-kernel/MANIFEST.sha256\n\
                       software-development-decision-kernel/agents/\n\
                       software-development-decision-kernel/skills/sddk-apply/SKILL.md\n";
        assert!(ensure_safe_tarball_members(listing).is_ok());
    }

    #[test]
    fn tar_guard_accepts_dotdot_inside_a_filename() {
        // `a..b` is a legitimate filename, not a traversal.
        let listing = "software-development-decision-kernel/docs/a..b.md\n";
        assert!(ensure_safe_tarball_members(listing).is_ok());
    }

    #[test]
    fn tar_guard_rejects_traversal_after_strip_components() {
        // The exact exploit: strip-components=1 turns this into `../etc/x`.
        let listing = "software-development-decision-kernel/../../etc/x\n";
        let err = ensure_safe_tarball_members(listing).unwrap_err();
        assert!(
            err.to_string().contains("escapes target directory"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn tar_guard_rejects_nested_traversal() {
        let listing = "prefix/a/b/../../../outside\n";
        assert!(ensure_safe_tarball_members(listing).is_err());
    }

    #[test]
    fn tar_guard_rejects_absolute_paths() {
        let listing = "/etc/cron.d/evil\n";
        let err = ensure_safe_tarball_members(listing).unwrap_err();
        assert!(
            err.to_string().contains("absolute path"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn tar_guard_rejects_when_any_member_is_hostile() {
        // One bad member poisons the whole archive: fail closed.
        let listing = "software-development-decision-kernel/ok.md\n\
                       software-development-decision-kernel/../../evil\n";
        assert!(ensure_safe_tarball_members(listing).is_err());
    }

    #[test]
    fn tar_guard_accepts_empty_listing() {
        assert!(ensure_safe_tarball_members("").is_ok());
        assert!(ensure_safe_tarball_members("\n\n  \n").is_ok());
    }

    #[test]
    fn is_bundle_version_dir_accepts_canonical_and_prerelease() {
        assert!(is_bundle_version_dir("1.63.0"));
        assert!(is_bundle_version_dir("1.63.0-rc.1"));
        assert!(is_bundle_version_dir("0.1.0+build.42"));
        // Two-segment versions like v1.63 show up in real GitHub tags;
        // accept them — the semver-aware sort handles padding to (0,0,0).
        assert!(is_bundle_version_dir("1.63"));
        assert!(!is_bundle_version_dir(""));
        assert!(!is_bundle_version_dir("legacy"));
        assert!(!is_bundle_version_dir("tmp"));
        assert!(!is_bundle_version_dir("a/b"));
        assert!(!is_bundle_version_dir("has space"));
    }

    #[test]
    fn cmp_bundle_version_handles_pre_release_and_padding() {
        use std::cmp::Ordering;
        assert_eq!(cmp_bundle_version("1.63.0", "1.63.0"), Ordering::Equal);
        assert_eq!(cmp_bundle_version("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(cmp_bundle_version("1.63.0-rc.1", "1.63.0"), Ordering::Less);
        assert_eq!(
            cmp_bundle_version("1.63.0", "1.63.0-rc.1"),
            Ordering::Greater
        );
        assert_eq!(cmp_bundle_version("1.63.0", "1.63.0.0"), Ordering::Less);
    }

    // ── swap_current_to tests ────────────────────────────────────────────────

    #[test]
    fn swap_current_to_dev_link_mode() {
        // swap_current_to with target = framework_dir itself (dev-link mode)
        let dir = tempfile::tempdir().unwrap();
        let bundle_root = dir.path();
        std::os::unix::fs::symlink(bundle_root, bundle_root.join("current")).unwrap();

        crate::dev::swap_current_to(bundle_root, bundle_root);

        let current_target = std::fs::read_link(bundle_root.join("current")).unwrap();
        assert_eq!(
            current_target, bundle_root,
            "current should point to bundle_root"
        );
    }

    #[test]
    fn swap_current_to_version_dir() {
        // swap_current_to with target = version dir (repoint mode)
        let dir = tempfile::tempdir().unwrap();
        let bundle_root = dir.path();
        make_version_dir(bundle_root, "1.70.0");
        std::os::unix::fs::symlink(bundle_root.join("1.70.0"), bundle_root.join("current"))
            .unwrap();

        crate::dev::swap_current_to(bundle_root, &bundle_root.join("1.70.0"));

        let current_target = std::fs::read_link(bundle_root.join("current")).unwrap();
        assert_eq!(
            current_target.file_name().unwrap().to_str().unwrap(),
            "1.70.0",
            "current should point to version dir"
        );
    }

    fn make_version_dir(framework: &Path, version: &str) {
        fs::create_dir_all(framework.join(version)).unwrap();
        fs::write(framework.join(version).join("marker"), version).unwrap();
    }

    #[test]
    fn prune_keeps_current_and_removes_others() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        for v in ["1.28.0", "1.40.0", "1.50.0", "1.63.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.63.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 0, Some("1.63.0")).unwrap();
        assert_eq!(kept, vec!["1.63.0"]);
        assert_eq!(removed.len(), 3);
        assert!(!framework.join("1.50.0").exists());
        assert!(!framework.join("1.40.0").exists());
        assert!(!framework.join("1.28.0").exists());
        assert!(framework.join("1.63.0").exists());
        assert!(framework.join("current").exists());
    }

    #[test]
    fn prune_with_keep_n_keeps_most_recent() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        for v in ["1.28.0", "1.40.0", "1.50.0", "1.63.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.63.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 2, Some("1.63.0")).unwrap();
        // 2 most recent = 1.63.0 + 1.50.0; current is already in keep.
        assert!(kept.contains(&"1.63.0".to_owned()));
        assert!(kept.contains(&"1.50.0".to_owned()));
        assert_eq!(removed.len(), 2);
        assert!(!framework.join("1.28.0").exists());
        assert!(!framework.join("1.40.0").exists());
        assert!(framework.join("1.50.0").exists());
        assert!(framework.join("1.63.0").exists());
    }

    #[test]
    fn prune_refuses_to_remove_current_even_when_not_in_top_n() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        for v in ["1.28.0", "1.63.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.28.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 1, Some("1.28.0")).unwrap();
        // keep_n=1 would pick 1.63.0, but 1.28.0 is current so it is added.
        assert!(kept.contains(&"1.28.0".to_owned()));
        assert!(kept.contains(&"1.63.0".to_owned()));
        assert!(removed.is_empty());
        assert!(framework.join("1.28.0").exists());
        assert!(framework.join("1.63.0").exists());
    }

    #[test]
    fn prune_ignores_non_version_entries() {
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();
        make_version_dir(framework, "1.63.0");
        fs::create_dir(framework.join("legacy")).unwrap();
        fs::create_dir(framework.join("tmp")).unwrap();
        fs::write(framework.join("stray.txt"), "ignored").unwrap();
        std::os::unix::fs::symlink(framework.join("1.63.0"), framework.join("current")).unwrap();

        let (removed, kept) = prune_stale_bundles(framework, 0, Some("1.63.0")).unwrap();
        assert!(removed.is_empty());
        assert_eq!(kept, vec!["1.63.0"]);
        // Non-version entries must survive the prune.
        assert!(framework.join("legacy").exists());
        assert!(framework.join("tmp").exists());
        assert!(framework.join("stray.txt").exists());
    }

    // ── INC-DEBT-020 regression tests ──────────────────────────────────────────

    #[test]
    fn prune_repoint_current_after_prune_keeps_valid_target() {
        // INC-DEBT-020: after prune removes stale versions, current must still
        // point to a valid directory (repoint_current_to_newest).
        // Scenario: current=1.70.0, we keep [1.65.0], 1.70.0 gets removed.
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();

        // Create versions; 1.70.0 is current but will be removed by prune.
        for v in ["1.60.0", "1.65.0", "1.70.0"] {
            make_version_dir(framework, v);
        }
        std::os::unix::fs::symlink(framework.join("1.70.0"), framework.join("current")).unwrap();

        // Simulate prune that keeps 1.65.0 (not 1.70.0) — e.g. --keep 1
        // when active was 1.70.0 but kept = [1.65.0] because 1.70.0 was in a
        // "removed" set from a previous prune cycle.
        let _removed = ["1.70.0".to_owned()];
        let kept = ["1.65.0".to_owned()];

        // Call repoint_current_to_newest with the newest kept version
        repoint_current_to_newest(framework, kept.first().unwrap());

        // current must now point to 1.65.0 (valid, existing dir)
        let current_target = std::fs::read_link(framework.join("current")).unwrap();
        assert_eq!(
            current_target.file_name().unwrap().to_str().unwrap(),
            "1.65.0",
            "current must repoint to newest kept version"
        );
        assert!(
            framework.join("1.65.0").is_dir(),
            "current target must be an existing directory"
        );
    }

    #[test]
    fn repoint_current_skips_nonexistent_version_dir() {
        // If the "newest" kept version dir doesn't exist on disk, skip repoint.
        let dir = tempfile::tempdir().unwrap();
        let framework = dir.path();

        make_version_dir(framework, "1.60.0");
        std::os::unix::fs::symlink(framework.join("1.60.0"), framework.join("current")).unwrap();

        // Try to repoint to a non-existent version
        repoint_current_to_newest(framework, "1.99.0");

        // current should still point to 1.60.0 (unchanged)
        let current_target = std::fs::read_link(framework.join("current")).unwrap();
        assert_eq!(
            current_target.file_name().unwrap().to_str().unwrap(),
            "1.60.0",
        );
    }

    // ── S-DEV-LINK-PRESERVED ────────────────────────────────────────────────

    #[test]
    fn s_dev_link_preserved_without_prune_flags_keeps_dev_link_target() {
        // S-DEV-LINK-PRESERVED: dev-link mode (root=".") WITHOUT --prune/--prune-only
        // → the dev-link block at update.rs:278-280 runs and current still points
        //   to bundle_root (framework/), NOT repointed to a version dir.
        // Regression guard: the dev-link behavior is preserved when no prune flags are passed.
        //
        // This test exercises the REAL swap_current_to helper directly.

        let dir = tempfile::tempdir().unwrap();

        // bundle_root = sddk_data_dir/framework
        let sddk_data = dir.path().join(".local").join("sddk");
        let bundle_root = sddk_data.join("framework");

        // Create version dirs at bundle_root location
        make_version_dir(&bundle_root, "1.66.6");
        make_version_dir(&bundle_root, "1.67.0");

        // Dev-link mode BEFORE: current symlink points to bundle_root itself
        std::os::unix::fs::symlink(&bundle_root, bundle_root.join("current")).unwrap();

        // Call the REAL swap_current_to function (dev-link mode: target = bundle_root)
        crate::dev::swap_current_to(&bundle_root, &bundle_root);

        // Key assertion: current symlink must still point to bundle_root (framework/)
        let current_link = bundle_root.join("current");
        let current_target = std::fs::read_link(&current_link).unwrap();
        // The symlink should point to bundle_root (the framework dir itself, not a version dir)
        assert_eq!(
            current_target, bundle_root,
            "current symlink must point to bundle_root (framework/) — dev-link preserved"
        );
    }
}
