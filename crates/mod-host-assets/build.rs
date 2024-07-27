fn main() {
    cxx_build::bridge("src/lib.rs")
        .file("src/dlstring.cpp")
        .include("include")
        .compile("test_project");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/dlheader.cpp");
    println!("cargo:rerun-if-changed=include/dlheader.h");
}
