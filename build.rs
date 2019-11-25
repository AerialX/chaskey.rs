const ROUNDS: usize = 12;

#[cfg(feature = "ffi")]
fn main() {
    println!("rerun-if-changed=build.rs");
    println!("rerun-if-changed=chaskey/chaskey.c");
    println!("cargo:rustc-cfg=chaskey_rounds=\"{}\"", ROUNDS);
    cc::Build::new()
        .file("chaskey/chaskey.c")
        .define("CHASKEY_ROUNDS", Some(&ROUNDS.to_string()[..]))
        .compile("chaskey");
}

#[cfg(not(feature = "ffi"))]
fn main() {
    println!("cargo:rustc-cfg=chaskey_rounds=\"{}\"", ROUNDS);
}
