#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

extern crate libloading;

use libloading::Library;

mod fbc {
    include!("./bindings/nv_fbc.rs");
}

mod enc {
    include!("./bindings/nv_enc.rs");
}

fn main() {
unsafe {
    let nv_fbc = fbc::NvFBC::new("/lib64/libnvidia-fbc.so.1").unwrap();   // TODO: Add proper library discovery.
    let nv_enc = enc::NvEnc::new("/lib64/libnvidia-encode.so.1").unwrap();//

    let mut pCapFn = std::mem::zeroed::<fbc::NVFBC_API_FUNCTION_LIST>();
    let mut pEncFn = std::mem::zeroed::<enc::NV_ENCODE_API_FUNCTION_LIST>();

    pCapFn.dwVersion = nvfbc_version();

    let mut fbc_status = nv_fbc.NvFBCCreateInstance(&mut pCapFn);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        panic!("Failed to create NvFBC instance. Status = {}, exiting", fbc_status as u32);
    }

    pEncFn.version = nvenc_struct_version(2);

    let mut enc_status = nv_enc.NvEncodeAPICreateInstance(&mut pEncFn);
    if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
        panic!("Failed to create NvEnc instance. Status = {}, exiting", enc_status as u32);
    }

    let mut fbc_create_handle_params = std::mem::zeroed::<fbc::NVFBC_CREATE_HANDLE_PARAMS>();
    fbc_create_handle_params.dwVersion = nvfbc_struct_version(std::mem::size_of::<fbc::NVFBC_CREATE_HANDLE_PARAMS>() as u32, 2);
    fbc_create_handle_params.bExternallyManagedContext = fbc::NVFBC_BOOL::NVFBC_TRUE;
    fbc_create_handle_params.glxCtx = todo!();
    fbc_create_handle_params.glxFBConfig = todo!();

    let mut fbc_handle = std::mem::zeroed::<fbc::NVFBC_SESSION_HANDLE>();
    fbc_status = pCapFn.nvFBCCreateHandle.unwrap()(&mut fbc_handle, &mut fbc_create_handle_params);
        if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        panic!("Failed to create NvFBC handle. Status = {}, exiting", fbc_status as u32);
    }

    println!("Created instances!");
}
}

fn nvfbc_version() -> u32 {
    fbc::NVFBC_VERSION_MINOR | (fbc::NVFBC_VERSION_MAJOR << 8)
}

fn nvenc_version() -> u32 {
    enc::NVENCAPI_MAJOR_VERSION | (enc::NVENCAPI_MINOR_VERSION << 24)
}

fn nvfbc_struct_version(type_size: u32, ver: u32) -> u32 {
    type_size | ((ver) << 16) | (nvfbc_version() << 24)
}

fn nvenc_struct_version(ver: u32) -> u32 {
    nvenc_version() | ((ver)<<16) | (0x7 << 28)
}
