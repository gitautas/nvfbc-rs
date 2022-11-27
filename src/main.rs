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
    /*
     * Dynamically load the NVidia libraries.
     */
    let nv_fbc = fbc::NvFBC::new("/lib/x86_64-linux-gnu/libnvidia-fbc.so.1").unwrap();   // TODO: Add proper library discovery.
    let nv_enc = enc::NvEnc::new("/lib/x86_64-linux-gnu/libnvidia-encode.so.1").unwrap();//

    let mut pCapFn = std::mem::zeroed::<fbc::NVFBC_API_FUNCTION_LIST>();
    let mut pEncFn = std::mem::zeroed::<enc::NV_ENCODE_API_FUNCTION_LIST>();

    /*
     * Create an NvFBC instance.
     *
     * API function pointers are accessible through pFn.
     */
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

    /*
     * Create a session handle that is used to identify the client.
     */
    let mut fbc_create_handle_params = std::mem::zeroed::<fbc::NVFBC_CREATE_HANDLE_PARAMS>();
    fbc_create_handle_params.dwVersion = nvfbc_struct_version::<fbc::NVFBC_CREATE_HANDLE_PARAMS>(2);

    let mut fbc_handle = std::mem::zeroed::<fbc::NVFBC_SESSION_HANDLE>();

    fbc_status = pCapFn.nvFBCCreateHandle.unwrap()(&mut fbc_handle, &mut fbc_create_handle_params);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        panic!("Failed to create NvFBC handle. Status = {}, exiting", fbc_status as u32);
    }

    /*
     * Get information about the state of the display driver.
     *
     * This call is optional but helps the application decide what it should
     * do.
     */
    let mut fbc_status_params = std::mem::zeroed::<fbc::NVFBC_GET_STATUS_PARAMS>();
    fbc_status_params.dwVersion = nvfbc_struct_version::<fbc::NVFBC_GET_STATUS_PARAMS>(2);

    fbc_status = pCapFn.nvFBCGetStatus.unwrap()(fbc_handle, &mut fbc_status_params);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        let error = std::ffi::CStr::from_ptr(pCapFn.nvFBCGetLastErrorStr.unwrap()(fbc_handle)).to_str().unwrap();
        panic!("{}", error);
    }

    if fbc_status_params.bCanCreateNow == fbc::NVFBC_BOOL::NVFBC_FALSE {
        panic!("It is not possible to create a capture session on this system");
    }

    /*
     * Create a capture session.
     */
    let frame_size = fbc::NVFBC_SIZE {
        w: 3440,
        h: 1440,
    };

    let mut fbc_create_capture_params = std::mem::zeroed::<fbc::NVFBC_CREATE_CAPTURE_SESSION_PARAMS>();
    fbc_create_capture_params.dwVersion = nvfbc_struct_version::<fbc::NVFBC_CREATE_CAPTURE_SESSION_PARAMS>(6);
    fbc_create_capture_params.eCaptureType = fbc::NVFBC_CAPTURE_TYPE::NVFBC_CAPTURE_SHARED_CUDA;
    fbc_create_capture_params.bWithCursor = fbc::NVFBC_BOOL::NVFBC_TRUE;
    fbc_create_capture_params.frameSize = frame_size;
    fbc_create_capture_params.eTrackingType = fbc::NVFBC_TRACKING_TYPE::NVFBC_TRACKING_DEFAULT;

    fbc_status = pCapFn.nvFBCCreateCaptureSession.unwrap()(fbc_handle, &mut fbc_create_capture_params);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        let error = std::ffi::CStr::from_ptr(pCapFn.nvFBCGetLastErrorStr.unwrap()(fbc_handle)).to_str().unwrap();
        panic!("{}", error);
    }

    /*
     * Set up the capture session.
     */
    let mut fbc_setup_params = std::mem::zeroed::<fbc::NVFBC_TOCUDA_SETUP_PARAMS>();
    fbc_setup_params.dwVersion = nvfbc_struct_version::<fbc::NVFBC_TOCUDA_SETUP_PARAMS>(1);
    fbc_setup_params.eBufferFormat = fbc::NVFBC_BUFFER_FORMAT::NVFBC_BUFFER_FORMAT_RGB;

    fbc_status = pCapFn.nvFBCToCudaSetUp.unwrap()(fbc_handle, &mut fbc_setup_params);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        let error = std::ffi::CStr::from_ptr(pCapFn.nvFBCGetLastErrorStr.unwrap()(fbc_handle)).to_str().unwrap();
        panic!("{}", error);
    }

    println!("I ran succesfully tf???");
}
}

fn nvfbc_version() -> u32 {
    fbc::NVFBC_VERSION_MINOR | (fbc::NVFBC_VERSION_MAJOR << 8)
}

fn nvenc_version() -> u32 {
    enc::NVENCAPI_MAJOR_VERSION | (enc::NVENCAPI_MINOR_VERSION << 24)
}

fn nvfbc_struct_version<T>(ver: u32) -> u32 {
    std::mem::size_of::<T>() as u32 | ((ver) << 16) | (nvfbc_version() << 24)
}

fn nvenc_struct_version(ver: u32) -> u32 {
    nvenc_version() | ((ver)<<16) | (0x7 << 28)
}
