use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=native/Calendar.swift");
    println!("cargo:rerun-if-changed=native/Info.plist");
    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "macos" {
        return;
    }
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let helper = out.join("TesseraCalendar");
    let arch = if env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "aarch64" {
        "arm64"
    } else {
        "x86_64"
    };
    let status = Command::new("xcrun")
        .arg("swiftc")
        .arg(root.join("native/Calendar.swift"))
        .args([
            "-O",
            "-target",
            &format!("{arch}-apple-macosx13.0"),
            "-module-cache-path",
        ])
        .arg(out.join("swift-cache"))
        .args(["-o"])
        .arg(&helper)
        .args([
            "-Xlinker",
            "-sectcreate",
            "-Xlinker",
            "__TEXT",
            "-Xlinker",
            "__info_plist",
            "-Xlinker",
        ])
        .arg(root.join("native/Info.plist"))
        .status()
        .expect("Xcode command line tools are required for EventKit");
    assert!(status.success(), "EventKit helper compilation failed");
    assert!(
        Command::new("codesign")
            .args([
                "--force",
                "--sign",
                "-",
                "--identifier",
                "dev.tessera.calendar"
            ])
            .arg(helper)
            .status()
            .expect("codesign required")
            .success(),
        "EventKit helper signing failed"
    );
}
