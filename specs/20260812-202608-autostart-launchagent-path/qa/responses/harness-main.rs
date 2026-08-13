//! QA harness — drives the SHIPPED `autostart.rs` logic (included verbatim below,
//! minus the one tauri-dependent function) against the REAL `auto-launch` 0.5.0
//! writer, under a fake $HOME so the developer's own login item is never touched.

include!("../shipped.rs");

use std::fs;
use std::os::unix::fs::PermissionsExt;

static mut FAILED: u32 = 0;
static mut RUN: u32 = 0;

fn check(id: &str, what: &str, ok: bool, detail: String) {
    unsafe {
        RUN += 1;
        if !ok {
            FAILED += 1;
        }
    }
    println!(
        "{} {} — {}\n      {}",
        if ok { "PASS" } else { "FAIL" },
        id,
        what,
        detail
    );
}

fn root() -> PathBuf {
    PathBuf::from("/tmp/sc-qa-harness/sandbox")
}

fn fake_home() -> PathBuf {
    root().join("home")
}

fn plist_file() -> PathBuf {
    fake_home().join("Library/LaunchAgents/Site Checker.plist")
}

/// Build a real keg: <prefix>/Cellar/site-checker/<version>/libexec/Site Checker.app/Contents/MacOS/site-checker
fn make_keg(prefix: &Path, version: &str) -> PathBuf {
    let exe = prefix
        .join("Cellar/site-checker")
        .join(version)
        .join("libexec/Site Checker.app/Contents/MacOS/site-checker");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    exe.canonicalize().unwrap()
}

/// Point <prefix>/opt/site-checker at <prefix>/Cellar/site-checker/<version>, like `brew link`.
fn link_opt(prefix: &Path, version: &str) {
    let opt = prefix.join("opt/site-checker");
    fs::create_dir_all(opt.parent().unwrap()).unwrap();
    let _ = fs::remove_file(&opt);
    std::os::unix::fs::symlink(prefix.join("Cellar/site-checker").join(version), &opt).unwrap();
}

fn manager_for(running: &Path) -> AutoLaunch {
    // Mirrors the shipped `manager()`: app_name = package name, desired_path of
    // the canonicalised running copy, launch-agent, no args.
    AutoLaunch::new(
        "Site Checker",
        &desired_path(running).to_string_lossy(),
        true,
        &[] as &[&str],
    )
}

fn main() {
    // Fake $HOME. `auto-launch` resolves ~/Library/LaunchAgents through
    // dirs::home_dir(), which reads $HOME on unix.
    let _ = fs::remove_dir_all(root());
    fs::create_dir_all(fake_home().join("Library/LaunchAgents")).unwrap();
    std::env::set_var("HOME", fake_home());

    let prefix = root().join("brew");
    fs::create_dir_all(&prefix).unwrap();
    let prefix = prefix.canonicalize().unwrap();

    // ---------------- H1: fresh Homebrew install registers the opt path ----------------
    let keg_100 = make_keg(&prefix, "1.0.0");
    link_opt(&prefix, "1.0.0");
    let desired = desired_path(&keg_100);
    let expected_opt = prefix.join("opt/site-checker/libexec/Site Checker.app/Contents/MacOS/site-checker");
    check(
        "H1  TC-001",
        "FR-001/FR-002 US1-2: a keg path registers the version-independent opt path",
        desired == expected_opt && !desired.to_string_lossy().contains("1.0.0"),
        format!("running={}\n      desired={}", keg_100.display(), desired.display()),
    );

    let al = manager_for(&keg_100);
    al.enable().unwrap();
    let written = fs::read_to_string(plist_file()).unwrap();
    let recorded = recorded_path(&written);
    check(
        "H7  TC-019",
        "Contract: the real auto-launch 0.5.0 template round-trips through recorded_path()",
        recorded.as_deref() == Some(desired.to_string_lossy().as_ref()),
        format!("plist under fake HOME={}\n      recorded={:?}", plist_file().display(), recorded),
    );
    check(
        "H7b TC-019",
        "Contract: the recorded path contains a space and survives the round trip",
        recorded.as_deref().is_some_and(|r| r.contains("Site Checker.app")),
        format!("recorded={:?}", recorded),
    );

    // ---------------- H2: the upgrade itself (SC-001) ----------------
    let keg_101 = make_keg(&prefix, "1.0.1");
    link_opt(&prefix, "1.0.1");
    fs::remove_dir_all(prefix.join("Cellar/site-checker/1.0.0")).unwrap();
    let after_upgrade = recorded_path(&fs::read_to_string(plist_file()).unwrap()).unwrap();
    let resolves = Path::new(&after_upgrade).exists();
    let resolves_to_new = Path::new(&after_upgrade)
        .canonicalize()
        .is_ok_and(|p| p == keg_101);
    check(
        "H2  TC-006",
        "SC-001 US1-1 (simulated): the registration survives an upgrade that deletes the old keg",
        resolves && resolves_to_new,
        format!(
            "recorded={after_upgrade}\n      exists={resolves}, resolves to 1.0.1={resolves_to_new}"
        ),
    );
    // Counterfactual: what today's (pre-fix) behaviour would have recorded.
    let old_behaviour = keg_100.to_string_lossy().to_string();
    check(
        "H2b TC-006",
        "Counterfactual: registering the running keg path (pre-fix behaviour) dangles after the upgrade",
        !Path::new(&old_behaviour).exists(),
        format!("pre-fix would have recorded={old_behaviour} (now missing)"),
    );

    // ---------------- H3: non-package copies are unchanged (FR-004, SC-004) ----------------
    let dev = root().join("dev/src-tauri/target/debug/site-checker");
    fs::create_dir_all(dev.parent().unwrap()).unwrap();
    fs::write(&dev, b"x").unwrap();
    let dev = dev.canonicalize().unwrap();
    check(
        "H3  TC-002",
        "FR-004/SC-004 US1-3: a dev build registers its own path, byte for byte",
        desired_path(&dev) == dev,
        format!("running={}\n      desired={}", dev.display(), desired_path(&dev).display()),
    );
    let apps = root().join("Applications/Site Checker.app/Contents/MacOS/site-checker");
    fs::create_dir_all(apps.parent().unwrap()).unwrap();
    fs::write(&apps, b"x").unwrap();
    let apps = apps.canonicalize().unwrap();
    check(
        "H3b TC-002",
        "FR-004: a hand-placed /Applications copy registers its own path",
        desired_path(&apps) == apps,
        format!("desired={}", desired_path(&apps).display()),
    );

    // ---------------- H4: fallback when the stable path does not resolve (FR-003) ----------------
    fs::remove_file(prefix.join("opt/site-checker")).unwrap(); // like `brew unlink`
    check(
        "H4  TC-003",
        "FR-003: with the opt link gone (brew unlink), the running keg path is registered",
        desired_path(&keg_101) == keg_101,
        format!("desired={}", desired_path(&keg_101).display()),
    );
    // opt exists but points at a different application
    let other = root().join("other/site-checker");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, b"x").unwrap();
    let decoy = prefix.join("opt/site-checker/libexec/Site Checker.app/Contents/MacOS");
    fs::create_dir_all(&decoy).unwrap();
    fs::write(decoy.join("site-checker"), b"different app").unwrap();
    check(
        "H4b TC-003",
        "FR-003: an opt path that resolves to a DIFFERENT application is rejected",
        desired_path(&keg_101) == keg_101,
        format!("desired={}", desired_path(&keg_101).display()),
    );
    fs::remove_dir_all(prefix.join("opt")).unwrap();
    link_opt(&prefix, "1.0.1");

    // ---------------- H5: an unrelated `Cellar` directory (edge case) ----------------
    let wine = root().join("home/Documents/Cellar/wine-notes/2024/notes/site-checker");
    fs::create_dir_all(wine.parent().unwrap()).unwrap();
    fs::write(&wine, b"x").unwrap();
    let wine = wine.canonicalize().unwrap();
    check(
        "H5  TC-004",
        "Edge: a user directory named `Cellar` never produces a bogus registration",
        desired_path(&wine) == wine,
        format!("running={}\n      desired={}", wine.display(), desired_path(&wine).display()),
    );

    // ---------------- H6: launched through the /Applications shortcut (edge case) ----------------
    let shortcut = root().join("Applications2/Site Checker.app");
    fs::create_dir_all(shortcut.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(
        prefix.join("opt/site-checker/libexec/Site Checker.app"),
        &shortcut,
    )
    .unwrap();
    let via_shortcut = shortcut
        .join("Contents/MacOS/site-checker")
        .canonicalize()
        .unwrap();
    check(
        "H6  TC-005",
        "Edge: launching through the /Applications shortcut records the identical opt path",
        via_shortcut == keg_101 && desired_path(&via_shortcut) == expected_opt,
        format!(
            "canonicalised={}\n      desired={}",
            via_shortcut.display(),
            desired_path(&via_shortcut).display()
        ),
    );

    // ---------------- H8: a stale registration repairs itself (FR-005, SC-002) ----------------
    let al = manager_for(&keg_101);
    al.enable().unwrap();
    let good = fs::read_to_string(plist_file()).unwrap();
    let stale = good.replace(
        &desired_path(&keg_101).to_string_lossy().to_string(),
        "/opt/homebrew/Cellar/site-checker/0.0.0/libexec/Site Checker.app/Contents/MacOS/site-checker",
    );
    fs::write(plist_file(), &stale).unwrap();
    let enabled_before = al.is_enabled().unwrap();
    repair_if_stale(&al, &plist_file());
    let after = fs::read_to_string(plist_file()).unwrap();
    check(
        "H8  TC-007",
        "FR-005/SC-002 US2-1: a stale registration is rewritten to the desired path",
        recorded_path(&after).as_deref() == Some(desired_path(&keg_101).to_string_lossy().as_ref()),
        format!("before={:?}\n      after ={:?}", recorded_path(&stale), recorded_path(&after)),
    );
    check(
        "H12 TC-011",
        "FR-009/SC-005: the registration stays enabled across the repair (checkbox unchanged)",
        enabled_before && al.is_enabled().unwrap() && plist_file().exists(),
        format!("is_enabled before={enabled_before}, after={}", al.is_enabled().unwrap()),
    );

    // ---------------- H10: a correct registration is left untouched (US2-3) ----------------
    fs::write(plist_file(), &good).unwrap();
    let before = fs::read(plist_file()).unwrap();
    repair_if_stale(&al, &plist_file());
    check(
        "H10 TC-009",
        "US2-3: a registration already naming the desired path is left byte-for-byte untouched",
        fs::read(plist_file()).unwrap() == before,
        format!("{} bytes, unchanged", before.len()),
    );

    // ---------------- H9: repair never creates a registration (FR-006) ----------------
    fs::remove_file(plist_file()).unwrap();
    repair_if_stale(&al, &plist_file());
    check(
        "H9  TC-008",
        "FR-006 US2-2: with no registration present, repair creates nothing",
        !plist_file().exists() && !al.is_enabled().unwrap(),
        format!("exists={}, is_enabled={}", plist_file().exists(), al.is_enabled().unwrap()),
    );

    // ---------------- H11: an unreadable registration is left alone (FR-007) ----------------
    for (name, bytes) in [
        ("empty file", b"".to_vec()),
        ("not a plist", b"not a plist at all".to_vec()),
        ("truncated xml", b"<?xml version=\"1.0\"?><plist><dict><key>ProgramArguments</key>".to_vec()),
        ("binary plist", vec![0x62, 0x70, 0x6c, 0x69, 0x73, 0x74, 0x30, 0x30, 0xff, 0x00, 0x01]),
        ("empty array", b"<key>ProgramArguments</key>\n<array></array>".to_vec()),
    ] {
        fs::write(plist_file(), &bytes).unwrap();
        repair_if_stale(&al, &plist_file());
        check(
            "H11 TC-010",
            &format!("FR-007 US2-4: an uninterpretable registration ({name}) is left untouched"),
            fs::read(plist_file()).unwrap() == bytes,
            format!("{} bytes in, {} bytes out, identical", bytes.len(), fs::read(plist_file()).unwrap().len()),
        );
    }

    // ---------------- H13: a failing rewrite cannot stop execution (FR-008, SC-006) ----------------
    // quickstart §2d done correctly: the FILE must be unwritable, not just the directory.
    fs::write(plist_file(), &stale).unwrap();
    fs::set_permissions(plist_file(), fs::Permissions::from_mode(0o400)).unwrap();
    let sha_before = fs::read(plist_file()).unwrap();
    repair_if_stale(&al, &plist_file()); // must not panic
    let sha_after = fs::read(plist_file()).unwrap();
    check(
        "H13 TC-012",
        "FR-008/SC-006: a rewrite that fails (read-only plist) is swallowed; execution continues",
        sha_after == sha_before,
        format!(
            "plist mode 400, repair returned normally, {} bytes unchanged (still the stale path)",
            sha_after.len()
        ),
    );
    fs::set_permissions(plist_file(), fs::Permissions::from_mode(0o644)).unwrap();
    // and the same with the directory unwritable, as the quickstart literally says
    fs::write(plist_file(), &stale).unwrap();
    fs::set_permissions(fake_home().join("Library/LaunchAgents"), fs::Permissions::from_mode(0o500)).unwrap();
    repair_if_stale(&al, &plist_file());
    let dir_ro_result = recorded_path(&fs::read_to_string(plist_file()).unwrap());
    fs::set_permissions(fake_home().join("Library/LaunchAgents"), fs::Permissions::from_mode(0o700)).unwrap();
    check(
        "H13b TC-012",
        "quickstart §2d as written (chmod 500 on the DIRECTORY) does not make the rewrite fail",
        dir_ro_result.as_deref() == Some(desired_path(&keg_101).to_string_lossy().as_ref()),
        format!(
            "with the directory read-only the rewrite still SUCCEEDED (recorded={dir_ro_result:?}) — \
             confirms implementation-notes: §2d must chmod the file, not the directory"
        ),
    );

    // ---------------- FR-012: nothing here ever deletes the registration ----------------
    check(
        "H14 TC-017",
        "FR-012: no path reached from startup removes the registration",
        plist_file().exists(),
        "the registration is still present after every repair scenario above".to_string(),
    );

    unsafe {
        println!("\n{} checks run, {} failed", RUN, FAILED);
        std::process::exit(if FAILED == 0 { 0 } else { 1 });
    }
}
