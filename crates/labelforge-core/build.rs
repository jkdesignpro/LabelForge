// Compiles the vendored Zint barcode library (BSD-3, vendor/zint) and the
// LabelForge shim (vendor/lf_zint.c) into the core crate.
fn main() {
    let mut b = cc::Build::new();
    b.define("ZINT_NO_PNG", None)
        .include("vendor")
        .include("vendor/zint")
        .warnings(false)
        .opt_level(2);
    if b.get_compiler().is_like_msvc() {
        b.define("_CRT_SECURE_NO_WARNINGS", None);
    }
    for entry in std::fs::read_dir("vendor/zint").unwrap() {
        let p = entry.unwrap().path();
        if p.extension().map_or(false, |e| e == "c") {
            println!("cargo:rerun-if-changed={}", p.display());
            b.file(p);
        }
    }
    b.file("vendor/lf_zint.c");
    println!("cargo:rerun-if-changed=vendor/lf_zint.c");
    b.compile("lfzint");
}
