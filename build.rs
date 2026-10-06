use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=compositor/keyrc-compositor.c");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("Cargo profile directory is an OUT_DIR ancestor");
    let output = profile_dir.join("keyrc-compositor");

    let status = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()))
        .args([
            "-std=c11",
            "-O2",
            "-D_GNU_SOURCE",
            "-DHAVE_REALLOCARRAY=1",
            "-Wall",
            "-Wextra",
            "-Wno-unused-parameter",
            "-Wno-unused-but-set-variable",
            "compositor/keyrc-compositor.c",
            "-o",
        ])
        .arg(&output)
        .args([
            "-lXcomposite",
            "-lXdamage",
            "-lXfixes",
            "-lXrender",
            "-lXext",
            "-lX11",
            "-lm",
        ])
        .status()
        .expect("failed to run the C compiler for keyrc-compositor");

    assert!(status.success(), "failed to build keyrc-compositor");
}
