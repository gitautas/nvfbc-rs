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
        .header("external/nvEncodeAPI.h")
        .dynamic_library_name("NvEnc")
        // .dynamic_link_require_all(true)
        .default_enum_style(bindgen::EnumVariation::Rust {non_exhaustive: false})
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings for NvEnc");

    let cuda = bindgen::Builder::default()
        .header("external/cuda.h")
        .dynamic_library_name("cuda")
        // .dynamic_link_require_all(true)
        .default_enum_style(bindgen::EnumVariation::Rust {non_exhaustive: false})
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings for cuda");

    cuda
        .write_to_file("src/bindings/cuda.rs")
        .expect("Couldn't write cuda bindings.");

    nv_fbc
        .write_to_file("src/bindings/nv_fbc.rs")
        .expect("Couldn't write NvFBC bindings.");

    nv_enc
        .write_to_file("src/bindings/nv_enc.rs")
        .expect("Couldn't write NvEnc bindings.");

    println!("cargo:rerun-if-changed=external/stdio_wrapper.h");
    bindgen::Builder::default()
        .header("external/stdio_wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings for stdio")
        .write_to_file("src/bindings/stdio.rs")
        .expect("Couldn't write stdio bindings!");
}
