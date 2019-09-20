fn main() {
    println!("rerun-if-changed=build.rs");
    println!("rerun-if-changed=chaskey/chaskey.c");
    let rounds = 12;
    println!("cargo:rustc-cfg=chaskey_rounds=\"{}\"", rounds);
    cc::Build::new()
        .file("chaskey/chaskey.c")
        .define("CHASKEY_ROUNDS", Some(&rounds.to_string()[..]))
        .compile("chaskey");
}
