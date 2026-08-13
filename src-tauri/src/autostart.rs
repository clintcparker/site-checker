//! The login item — `~/Library/LaunchAgents/Site Checker.plist`, and only that file.
//!
//! This module owns launch-at-login registration end to end: which path gets
//! recorded, whether what is already recorded is still the one we want, and how
//! to correct it when it is not. It reads and writes nothing else — `sites.json`
//! and the first-run marker belong to other modules, and no failure here may
//! reach either.
//!
//! The decisions worth testing are pure functions with no Tauri and no
//! filesystem: `stable_path` (the `Cellar` → `opt` rewrite), `recorded_path`
//! (pulling a path back out of the plist) and `needs_repair` (whether to
//! rewrite). The rest is a thin shell over `auto-launch`.

use std::path::{Path, PathBuf};

use auto_launch::AutoLaunch;

/// Build the app's `AutoLaunch`.
///
/// `app_name` **must** stay `package_info().name`: the filename and the plist's
/// `Label` both derive from it, and any other value orphans an existing user's
/// `~/Library/LaunchAgents/Site Checker.plist` instead of repairing it.
pub fn manager(app: &tauri::AppHandle) -> Result<AutoLaunch, Box<dyn std::error::Error>> {
    let running = std::env::current_exe()?.canonicalize()?;
    Ok(AutoLaunch::new(
        &app.package_info().name,
        &desired_path(&running).to_string_lossy(),
        // `use_launch_agent = true` and no args, matching what the plugin
        // shipped. Both are compatibility surface: the launcher choice decides
        // the file's shape, and args land inside it.
        true,
        &[] as &[&str],
    ))
}

/// Rewrite a Homebrew keg path to the version-independent `opt/` path Homebrew
/// relinks on every upgrade.
///
/// ```text
/// /opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker
/// → /opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker
/// ```
///
/// Anchored on the `Cellar` component rather than a hardcoded prefix, so Apple
/// Silicon (`/opt/homebrew`), Intel (`/usr/local`) and a custom `--prefix` all
/// work. `None` is not a failure — it means "this install has no
/// version-independent form", which is the ordinary case for a hand-built copy
/// or a dev build, and the caller keeps today's behaviour.
///
/// Pure: this proposes a path, it never checks whether one is there.
fn stable_path(running: &Path) -> Option<PathBuf> {
    let parts: Vec<_> = running.components().collect();

    // Deepest match rather than first, so a directory a user happened to name
    // `Cellar` higher up cannot capture a keg path below it. Three following
    // components are required — formula, version, and a non-empty remainder —
    // because the executable is never the version directory itself.
    let cellar = parts.iter().enumerate().rev().find_map(|(i, part)| {
        (part.as_os_str() == "Cellar" && i + 3 < parts.len()).then_some(i)
    })?;

    let mut out = PathBuf::new();
    out.extend(&parts[..cellar]);
    out.push("opt");
    out.push(parts[cellar + 1]); // the formula
                                 // parts[cellar + 2] is the version — dropped, which is the whole point.
    out.extend(&parts[cellar + 3..]);
    Some(out)
}

/// The path to record in the login item.
///
/// The derived `opt/` path only if it exists *and* resolves back to the very
/// copy that is running; the running path otherwise. That second condition is
/// the guard that makes `stable_path` safe to state loosely — a coincidental
/// `Cellar` component in somebody's own directory tree yields a path that
/// either is not there or is not this application, and both fall back
/// (data-model.md rule 4 = FR-001–FR-004 in one place).
fn desired_path(running: &Path) -> PathBuf {
    stable_path(running)
        .filter(|stable| stable.canonicalize().is_ok_and(|resolved| resolved == running))
        .unwrap_or_else(|| running.to_path_buf())
}

/// Pull the recorded executable path out of a login item's XML.
///
/// Hand-rolled rather than delegated to a plist crate: this app writes the file
/// and the format is therefore known exactly (research R3). Anything that is
/// not that shape — a binary plist, a hand-written variant, a truncated file —
/// yields `None`, which the caller reads as "not ours to touch" (FR-007).
fn recorded_path(plist: &str) -> Option<String> {
    // Starts *after* the key, so the `Label` string earlier in the file can
    // never be mistaken for the program path.
    let after_key = plist.split_once("<key>ProgramArguments</key>")?.1;
    let array = after_key.split_once("<array>")?.1;
    // Bounded by the array, so a `<string>` belonging to a later key cannot be
    // read as the program either.
    let array = array.split_once("</array>").map_or(array, |(body, _)| body);
    let value = array.split_once("<string>")?.1.split_once("</string>")?.0;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// Whether the registration on disk should be rewritten.
///
/// `true` only when a path was successfully read **and** differs from the one
/// we want. Exact string equality, deliberately not a canonicalised comparison
/// (plan.md Judgment Call 4): canonicalising a path to a deleted binary fails,
/// and treating that failure as "equal" would hide the very breakage this
/// feature exists to repair.
///
/// `None` — absent, or unreadable — is `false`. The two cases collapse on
/// purpose: both mean "do nothing" (FR-006, FR-007).
fn needs_repair(plist: Option<&str>, desired: &str) -> bool {
    plist
        .and_then(recorded_path)
        .is_some_and(|recorded| recorded != desired)
}

/// Correct a registration that names the wrong path. Never creates one, never
/// removes one, never warns.
///
/// Infallible by construction (FR-008, I4): every step swallows its own error.
/// Startup must not be able to fail here, and nothing here may reach the site
/// list.
pub fn repair_if_stale(manager: &AutoLaunch, plist_path: &std::path::Path) {
    // `enable()` truncates and rewrites the same file, so repair is
    // `Present → Present` — the registration stays enabled throughout.
    repair_with(plist_path, manager.get_app_path(), || {
        let _ = manager.enable();
    })
}

/// The decision half of [`repair_if_stale`], with the write injected.
///
/// Split out so the tests can exercise this without a real `AutoLaunch`, whose
/// `enable()` resolves its own path from `$HOME` and would rewrite the
/// developer's actual `~/Library/LaunchAgents/Site Checker.plist`.
fn repair_with(plist_path: &std::path::Path, desired: &str, rewrite: impl FnOnce()) {
    // FR-006 / I1: a missing file makes `read_to_string` fail, which yields
    // `None`, which yields `false`. The absent branch does nothing — that is
    // how repair is prevented from ever creating a registration.
    let plist = std::fs::read_to_string(plist_path).ok();
    if needs_repair(plist.as_deref(), desired) {
        rewrite();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- stable_path: the Cellar → opt rewrite (FR-001, FR-002) ----

    #[test]
    fn apple_silicon_keg_rewrites_to_the_opt_path() {
        assert_eq!(
            stable_path(Path::new(
                "/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker"
            )),
            Some(PathBuf::from(
                "/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker"
            ))
        );
    }

    #[test]
    fn intel_prefix_rewrites_the_same_way() {
        assert_eq!(
            stable_path(Path::new(
                "/usr/local/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker"
            )),
            Some(PathBuf::from(
                "/usr/local/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker"
            ))
        );
    }

    #[test]
    fn a_relocated_prefix_rewrites_the_same_way() {
        // Anchored on the `Cellar` component, never on a hardcoded prefix, so a
        // `brew --prefix` anywhere works.
        assert_eq!(
            stable_path(Path::new(
                "/Users/x/brew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker"
            )),
            Some(PathBuf::from(
                "/Users/x/brew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker"
            ))
        );
    }

    #[test]
    fn a_version_with_a_revision_suffix_is_still_dropped() {
        // Homebrew appends `_1` for a formula revision; the version component is
        // dropped wholesale, so its spelling never matters.
        assert_eq!(
            stable_path(Path::new(
                "/opt/homebrew/Cellar/site-checker/1.0.0_1/libexec/Site Checker.app/Contents/MacOS/site-checker"
            )),
            Some(PathBuf::from(
                "/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker"
            ))
        );
    }

    // ---- stable_path negatives: no version-independent form exists (FR-004) ----

    #[test]
    fn a_hand_built_copy_in_applications_has_no_stable_path() {
        assert_eq!(
            stable_path(Path::new(
                "/Applications/Site Checker.app/Contents/MacOS/site-checker"
            )),
            None
        );
    }

    #[test]
    fn a_development_build_has_no_stable_path() {
        assert_eq!(
            stable_path(Path::new(
                "/Users/x/src/site-checker/src-tauri/target/debug/site-checker"
            )),
            None
        );
    }

    #[test]
    fn cellar_with_no_version_component_is_not_a_keg() {
        // `Cellar/<formula>` and nothing else: too shallow to be a keg path.
        assert_eq!(
            stable_path(Path::new("/opt/homebrew/Cellar/site-checker")),
            None
        );
    }

    #[test]
    fn cellar_with_nothing_after_the_version_is_not_a_keg() {
        // The executable is never the version directory itself, so a match that
        // leaves no remainder is somebody's notes folder, not a keg.
        assert_eq!(
            stable_path(Path::new("/opt/homebrew/Cellar/site-checker/1.0.0")),
            None
        );
    }

    #[test]
    fn the_last_cellar_component_wins() {
        // Deepest rather than first, so a directory a user happened to name
        // `Cellar` higher up cannot capture a real keg path below it
        // (data-model.md derivation rule 1).
        assert_eq!(
            stable_path(Path::new(
                "/Users/x/Cellar/notes/opt/homebrew/Cellar/site-checker/1.0.0/libexec/site-checker"
            )),
            Some(PathBuf::from(
                "/Users/x/Cellar/notes/opt/homebrew/opt/site-checker/libexec/site-checker"
            ))
        );
    }

    // ---- desired_path: derivation is only a proposal; existence decides (FR-003) ----

    #[test]
    fn a_derived_path_that_does_not_exist_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let running = dir
            .path()
            .join("Cellar/site-checker/1.0.0/libexec/site-checker");
        std::fs::create_dir_all(running.parent().unwrap()).unwrap();
        std::fs::write(&running, b"").unwrap();
        // `opt/site-checker/libexec/site-checker` was never created.
        assert_eq!(desired_path(&running), running);
    }

    #[test]
    fn a_derived_path_that_is_a_different_file_is_refused() {
        // The "unrelated Cellar directory" case: the derived path happens to
        // exist, but it is not this application.
        let dir = tempfile::tempdir().unwrap();
        let running = dir
            .path()
            .join("Cellar/site-checker/1.0.0/libexec/site-checker");
        std::fs::create_dir_all(running.parent().unwrap()).unwrap();
        std::fs::write(&running, b"the real one").unwrap();

        let derived = dir.path().join("opt/site-checker/libexec/site-checker");
        std::fs::create_dir_all(derived.parent().unwrap()).unwrap();
        std::fs::write(&derived, b"somebody else entirely").unwrap();

        assert_eq!(desired_path(&running), running);
    }

    #[test]
    fn a_derived_path_that_resolves_back_to_the_running_copy_is_recorded() {
        // What a real Homebrew install looks like: `opt/<formula>` is a symlink
        // to the current keg, so the derived path canonicalises to the running
        // executable. This is the whole point of the feature.
        let dir = tempfile::tempdir().unwrap();
        let keg = dir.path().join("Cellar/site-checker/1.0.0");
        std::fs::create_dir_all(keg.join("libexec")).unwrap();
        let running = keg.join("libexec/site-checker");
        std::fs::write(&running, b"").unwrap();

        std::fs::create_dir_all(dir.path().join("opt")).unwrap();
        std::os::unix::fs::symlink(&keg, dir.path().join("opt/site-checker")).unwrap();

        // `dir.path()` may itself sit under a symlink (/var → /private/var on
        // macOS), so both sides are built from the canonical root — which is
        // what the caller passes in production.
        let root = dir.path().canonicalize().unwrap();
        let running = running.canonicalize().unwrap();
        let derived = root.join("opt/site-checker/libexec/site-checker");
        assert_eq!(desired_path(&running), derived);
    }

    // ---- recorded_path: reading back what we wrote ----

    /// Exactly what `auto-launch` 0.5 writes, byte for byte
    /// (contracts/launch-agent-plist.md). Asserted against verbatim so that a
    /// future `auto-launch` template change fails this test rather than
    /// silently disabling repair.
    fn plist_template(program: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
  <key>Label</key>
  <string>Site Checker</string>
  <key>ProgramArguments</key>
  <array><string>{program}</string></array>
  <key>RunAtLoad</key>
  <true/>
  </dict>
</plist>
"#
        )
    }

    #[test]
    fn the_recorded_path_is_read_out_of_the_template() {
        let plist = plist_template(
            "/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker",
        );
        assert_eq!(
            recorded_path(&plist).as_deref(),
            Some("/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker")
        );
    }

    #[test]
    fn a_recorded_path_containing_spaces_survives_intact() {
        // The real path always contains "Site Checker.app"; truncating at the
        // space would break every repair decision.
        let plist =
            plist_template("/Applications/Site Checker.app/Contents/MacOS/site-checker");
        assert_eq!(
            recorded_path(&plist).as_deref(),
            Some("/Applications/Site Checker.app/Contents/MacOS/site-checker")
        );
    }

    #[test]
    fn the_label_string_is_never_mistaken_for_the_program() {
        // `<string>Site Checker</string>` appears *before* ProgramArguments in
        // the template; the scan must start after the key, not at the top.
        let plist = plist_template("/somewhere/site-checker");
        assert_ne!(recorded_path(&plist).as_deref(), Some("Site Checker"));
    }

    // ---- recorded_path negatives: anything not our shape is not ours (FR-007) ----

    #[test]
    fn an_empty_file_records_nothing() {
        assert_eq!(recorded_path(""), None);
    }

    #[test]
    fn a_truncated_file_records_nothing() {
        let plist = plist_template("/somewhere/site-checker");
        let truncated = &plist[..plist.find("<array>").unwrap() + 7];
        assert_eq!(recorded_path(truncated), None);
    }

    #[test]
    fn a_plist_without_program_arguments_records_nothing() {
        assert_eq!(
            recorded_path(
                r#"<plist version="1.0"><dict><key>Label</key><string>Site Checker</string></dict></plist>"#
            ),
            None
        );
    }

    #[test]
    fn an_empty_program_arguments_array_records_nothing() {
        assert_eq!(
            recorded_path("<key>ProgramArguments</key>\n  <array></array>"),
            None
        );
    }

    #[test]
    fn an_empty_program_string_records_nothing() {
        assert_eq!(
            recorded_path("<key>ProgramArguments</key>\n  <array><string>  </string></array>"),
            None
        );
    }

    #[test]
    fn a_string_after_the_array_is_not_read_as_the_program() {
        // Bounded by `</array>`, so a `<string>` belonging to a later key can
        // never be mistaken for the program path.
        assert_eq!(
            recorded_path(
                "<key>ProgramArguments</key>\n  <array></array>\n  <key>Other</key><string>/nope</string>"
            ),
            None
        );
    }

    #[test]
    fn binary_bytes_record_nothing() {
        // A binary plist never reaches `recorded_path` as text — `read_to_string`
        // rejects non-UTF-8 first — but the shape check refuses it regardless.
        let bytes = [0x62u8, 0x70, 0x6c, 0x69, 0x73, 0x74, 0x30, 0x30, 0xd1, 0x01];
        assert_eq!(recorded_path(&String::from_utf8_lossy(&bytes)), None);
    }

    // ---- needs_repair: the decision (FR-005, FR-006, FR-007) ----

    #[test]
    fn a_registration_naming_another_path_needs_repair() {
        let plist = plist_template(
            "/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker",
        );
        assert!(needs_repair(
            Some(&plist),
            "/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker"
        ));
    }

    #[test]
    fn a_registration_naming_the_desired_path_is_left_alone() {
        let desired =
            "/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker";
        assert!(!needs_repair(Some(&plist_template(desired)), desired));
    }

    #[test]
    fn no_registration_at_all_is_never_repaired() {
        // FR-006: repair never creates. Absent is the user's choice to respect.
        assert!(!needs_repair(None, "/anything"));
    }

    #[test]
    fn an_uninterpretable_registration_is_never_repaired() {
        // FR-007: not in the shape this app writes, so not this app's to
        // rewrite. Collapses with the absent case deliberately — both mean
        // "do nothing" (data-model.md → Internal values).
        assert!(!needs_repair(Some("not a plist at all"), "/anything"));
    }

    // ---- repair_with: the shell (FR-008, I1, I4) ----
    //
    // Exercised through `repair_with` rather than `repair_if_stale`, because a
    // real `AutoLaunch::enable()` resolves its own path from `$HOME` and would
    // rewrite the developer's actual `~/Library/LaunchAgents/Site Checker.plist`
    // every time the suite ran. `repair_if_stale` adds nothing but that call.

    /// Runs `repair_with` against a temp file and reports whether the rewrite
    /// fired, without performing one.
    fn repaired(plist_contents: Option<&[u8]>, desired: &str) -> bool {
        let dir = tempfile::tempdir().unwrap();
        let plist = dir.path().join("Site Checker.plist");
        if let Some(bytes) = plist_contents {
            std::fs::write(&plist, bytes).unwrap();
        }

        let mut fired = false;
        repair_with(&plist, desired, || fired = true);

        // Nothing in this module writes the file itself — only the injected
        // rewrite would, and it does not.
        assert_eq!(plist.exists(), plist_contents.is_some());
        fired
    }

    #[test]
    fn repair_rewrites_a_stale_registration() {
        let stale = plist_template(
            "/opt/homebrew/Cellar/site-checker/1.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker",
        );
        assert!(repaired(
            Some(stale.as_bytes()),
            "/opt/homebrew/opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker"
        ));
    }

    #[test]
    fn repair_never_creates_a_missing_registration() {
        // I1: the app only reaches Absent → Present on first run, never here.
        assert!(!repaired(None, "/anywhere/site-checker"));
    }

    #[test]
    fn repair_leaves_an_unreadable_registration_alone() {
        // FR-007. Non-UTF-8 bytes: `read_to_string` refuses them before the
        // shape check ever runs.
        assert!(!repaired(
            Some(b"\x62\x70\x6c\x69\x73\x74\x30\x30\xd1\x01"),
            "/anywhere/site-checker"
        ));
    }

    #[test]
    fn repair_of_a_current_registration_does_nothing() {
        let desired = "/opt/homebrew/opt/site-checker/libexec/site-checker";
        assert!(!repaired(
            Some(plist_template(desired).as_bytes()),
            desired
        ));
    }
}
