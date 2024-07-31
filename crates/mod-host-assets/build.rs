fn main() {
    cxx_build::bridge("src/lib.rs")
        .include("include")
        .compile("test");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=include/dlstring.h");
}
