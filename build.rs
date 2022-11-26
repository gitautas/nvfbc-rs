extern crate bindgen;

fn main() {
    let nv_fbc = bindgen::Builder::default()
        .header("external/NvFBC.h")
        .dynamic_library_name("NvFBC")
        .dynamic_link_require_all(true)
        .default_enum_style(bindgen::EnumVariation::Rust {non_exhaustive: false})
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings for NvFBC");

    let nv_enc = bindgen::Builder::default()
        .header("external/NvEncodeAPI.h")
        .dynamic_library_name("NvEnc")
        // .dynamic_link_require_all(true)
        .default_enum_style(bindgen::EnumVariation::Rust {non_exhaustive: false})
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings for NvEnc");

    nv_fbc
        .write_to_file("src/bindings/nv_fbc.rs")
        .expect("Couldn't write NvFBC bindings.");

    nv_enc
        .write_to_file("src/bindings/nv_enc.rs")
        .expect("Couldn't write NvEnc bindings.");
}
