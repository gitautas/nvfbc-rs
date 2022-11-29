#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]

mod fbc {include!("./bindings/nv_fbc.rs");}
mod enc {include!("./bindings/nv_enc.rs");}
mod cuda {include!("./bindings/cuda.rs");}
mod stdio {include!("./bindings/stdio.rs");}

extern crate libloading;
use libloading::Library;

use crate::enc::{NV_ENC_PRESET_LOW_LATENCY_DEFAULT_GUID, NV_ENC_PRESET_LOW_LATENCY_HP_GUID, NV_ENC_PRESET_DEFAULT_GUID};

fn main() {
unsafe {
    /*
     * Dynamically load the NVidia libraries.
     */
    let nv_fbc = fbc::NvFBC::new("/lib/x86_64-linux-gnu/libnvidia-fbc.so.1").unwrap();   // TODO: Add proper library discovery.
    let nv_enc = enc::NvEnc::new("/lib/x86_64-linux-gnu/libnvidia-encode.so.1").unwrap();//
    let nv_cuda = cuda::cuda::new("/lib/x86_64-linux-gnu/libcuda.so.1").unwrap();

    /*
     * Initialize CUDA. 
     */
    let mut cu_ctx = std::mem::zeroed::<cuda::CUcontext>();
    let mut cu_dev = std::mem::zeroed::<cuda::CUdevice>();

    let mut cu_res = nv_cuda.cuInit(0);
    if cu_res != cuda::cudaError_enum::CUDA_SUCCESS {
        panic!("Unable to initialize CUDA context. Result: {}", cu_res as u32);
    }

    cu_res = nv_cuda.cuDeviceGet(&mut cu_dev, 0);
    if cu_res != cuda::cudaError_enum::CUDA_SUCCESS {
        panic!("Unable to get CUDA device. Result: {}", cu_res as u32);
    }

    cu_res = nv_cuda.cuCtxCreate_v2(&mut cu_ctx, cuda::CUctx_flags::CU_CTX_SCHED_AUTO as u32, cu_dev);
    if cu_res != cuda::cudaError_enum::CUDA_SUCCESS {
        panic!("Unable to create CUDA context. Result: {}", cu_res as u32);
    }
    
    /*
     * Create an NvFBC instance.
     *
     * API function pointers are accessible through cap_fn.
     */
    let mut cap_fn = std::mem::zeroed::<fbc::NVFBC_API_FUNCTION_LIST>();
    cap_fn.dwVersion = nvfbc_version();
    
    let mut fbc_status = nv_fbc.NvFBCCreateInstance(&mut cap_fn);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        panic!("Failed to create NvFBC instance. Status = {}, exiting", fbc_status as u32);
    }

    /*
     * Create an NvEnc instance.
     *
     * API function pointers are accesible through enc_fn.
     */
    let mut enc_fn = std::mem::zeroed::<enc::NV_ENCODE_API_FUNCTION_LIST>();
    enc_fn.version = nvenc_struct_version(2);

    let mut enc_status = nv_enc.NvEncodeAPICreateInstance(&mut enc_fn);
    if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
        panic!("Failed to create NvEnc instance. Status = {}, exiting", enc_status as u32);
    }

    /*
     * Create a session handle that is used to identify the client.
     */
    let mut fbc_create_handle_params = std::mem::zeroed::<fbc::NVFBC_CREATE_HANDLE_PARAMS>();
    fbc_create_handle_params.dwVersion = nvfbc_struct_version::<fbc::NVFBC_CREATE_HANDLE_PARAMS>(2);

    let mut fbc_handle = std::mem::zeroed::<fbc::NVFBC_SESSION_HANDLE>();

    fbc_status = cap_fn.nvFBCCreateHandle.unwrap()(&mut fbc_handle, &mut fbc_create_handle_params);
    if fbc_status == fbc::NVFBCSTATUS::NVFBC_ERR_UNSUPPORTED {
        println!("Your hardware doesn't support NvFBC or is unpatched");
        println!("Ensure you have a supported GPU and if you have a consumer level GPU, apply this patch:");
        println!("      https://github.com/keylase/nvidia-patch");
        println!("(please make sure to apply patch-fbc.sh)");
    }

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

    fbc_status = cap_fn.nvFBCGetStatus.unwrap()(fbc_handle, &mut fbc_status_params);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        let error = std::ffi::CStr::from_ptr(cap_fn.nvFBCGetLastErrorStr.unwrap()(fbc_handle)).to_str().unwrap();
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

    fbc_status = cap_fn.nvFBCCreateCaptureSession.unwrap()(fbc_handle, &mut fbc_create_capture_params);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        let error = std::ffi::CStr::from_ptr(cap_fn.nvFBCGetLastErrorStr.unwrap()(fbc_handle)).to_str().unwrap();
        panic!("{}", error);
    }

    /*
     * Set up the capture session.
     */
    let mut fbc_setup_params = std::mem::zeroed::<fbc::NVFBC_TOCUDA_SETUP_PARAMS>();
    fbc_setup_params.dwVersion = nvfbc_struct_version::<fbc::NVFBC_TOCUDA_SETUP_PARAMS>(1);
    fbc_setup_params.eBufferFormat = fbc::NVFBC_BUFFER_FORMAT::NVFBC_BUFFER_FORMAT_NV12;

    fbc_status = cap_fn.nvFBCToCudaSetUp.unwrap()(fbc_handle, &mut fbc_setup_params);
    if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
        let error = std::ffi::CStr::from_ptr(cap_fn.nvFBCGetLastErrorStr.unwrap()(fbc_handle)).to_str().unwrap();
        panic!("{}", error);
    }

    /*
     * Create an encoder session.
     */
    let mut enc_session_params = std::mem::zeroed::<enc::NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS>();
    enc_session_params.version = nvenc_struct_version(1);
    enc_session_params.apiVersion = enc::NVENCAPI_VERSION;
    enc_session_params.deviceType = enc::NV_ENC_DEVICE_TYPE::NV_ENC_DEVICE_TYPE_CUDA;
    enc_session_params.device = cu_ctx as *mut std::ffi::c_void;

    let mut encoder = std::ptr::null_mut();

    enc_status = enc_fn.nvEncOpenEncodeSessionEx.unwrap()(&mut enc_session_params, &mut encoder);
    if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
        panic!("Failed to open an encoder session. Status = {}", enc_status as u32);
    }

    /*
     * Validate the codec.
     */

     //{ 0x6bc82762, 0x4e63, 0x4ca4, { 0xaa, 0x85, 0x1e, 0x50, 0xf3, 0x21, 0xf6, 0xbf } };
    const codec_h264: enc::_GUID = enc::_GUID {
        Data1: 0x6bc82762,
        Data2: 0x4e63,
        Data3: 0x4ca4,
        Data4: [0xaa, 0x85, 0x1e, 0x50, 0xf3, 0x21, 0xf6, 0xbf ]
    };

    const preset_low_latency: enc::_GUID = enc::_GUID {
        Data1: 0x49df21c5,
        Data2: 0x6dfa,
        Data3: 0x4feb,
        Data4: [0x97, 0x87, 0x6a, 0xcc, 0x9e, 0xff, 0xb7, 0x26]
    };

    // let mut enc_guid_count = 0;
    // enc_status = enc_fn.nvEncGetEncodeGUIDCount.unwrap()(encoder, &mut enc_guid_count);
    // if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
    //     panic!("Failed to query number of supported codecs. Status = {}", enc_status as u32);
    // }
    
    // let mut enc_guid_array = Vec::<enc::GUID>::with_capacity(enc_guid_count as usize);
    // let mut enc_nguids = 0;

    // enc_status = enc_fn.nvEncGetEncodeGUIDs.unwrap()(encoder, enc_guid_array.as_mut_ptr(), enc_guid_count, &mut enc_nguids);
    // if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
    //     panic!("Failed to query number of supported codecs. Status = {}", enc_status as u32);
    // }
    
    // let mut codec_found = false;

    // for i in 0..enc_nguids {
    //     if codec_h264.Data1 == enc_guid_array[i as usize].Data1 &&
    //        codec_h264.Data2 == enc_guid_array[i as usize].Data2 &&
    //        codec_h264.Data3 == enc_guid_array[i as usize].Data3 &&
    //        codec_h264.Data4 == enc_guid_array[i as usize].Data4 
    //     {
    //         codec_found = true;
    //         break;
    //     }
    // }

    // if !codec_found {
    //     panic!("Could not enumerate the H264 codec");
    // }

    /*
     * Initialize the encoder preset configuration.
     */
    let mut enc_preset_config = std::mem::MaybeUninit::<enc::NV_ENC_PRESET_CONFIG>::zeroed().assume_init();
    enc_preset_config.version = nvenc_struct_version(4) | (1<<31);
    enc_preset_config.presetCfg.version = nvenc_struct_version(6) | (1<<31);
    
    enc_status = enc_fn.nvEncGetEncodePresetConfig.unwrap()(encoder, codec_h264, preset_low_latency, &mut enc_preset_config);
    if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
        panic!("Failed to obtain encoder preset settings. Status = {}", enc_status as u32);
    }

    enc_preset_config.presetCfg.rcParams.averageBitRate = 5 * 1024 * 1024;
    enc_preset_config.presetCfg.rcParams.maxBitRate = 8 * 1024 * 1024;

    /*
     * Initialize the encoder.
     */
    let mut enc_init_params = std::mem::MaybeUninit::<enc::NV_ENC_INITIALIZE_PARAMS>::zeroed().assume_init();
    
    enc_init_params.version = nvenc_struct_version(5) | ( 1<<31 );
    enc_init_params.encodeGUID = codec_h264;
    enc_init_params.presetGUID = preset_low_latency;
    enc_init_params.encodeConfig = &mut enc_preset_config.presetCfg;
    enc_init_params.encodeWidth = frame_size.w;
    enc_init_params.encodeHeight = frame_size.h;
    enc_init_params.frameRateNum = 100;
    enc_init_params.frameRateDen = 1;
    enc_init_params.enablePTD = 1; // This feature causes a lot of latency from my experience

    enc_status = enc_fn.nvEncInitializeEncoder.unwrap()(encoder, &mut enc_init_params);
    if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
        panic!("Failed to initialize the encode session. Status = {}", enc_status as u32);
    }

    /*
     * Create a buffer to hold the frame.
     */
    // let mut cu_dev_ptr = nv_cuda.cuDevice

    let mut pitch = frame_size.w as usize;

    let mut buffer = std::mem::MaybeUninit::<cuda::CUdeviceptr>::zeroed().assume_init();

    let mut enc_input_buffer_params = std::mem::MaybeUninit::<enc::NV_ENC_CREATE_INPUT_BUFFER>::zeroed().assume_init();
    enc_input_buffer_params.version = nvenc_struct_version(1);
    enc_input_buffer_params.width = frame_size.w;
    enc_input_buffer_params.height = frame_size.h;
    enc_input_buffer_params.bufferFmt = enc::NV_ENC_BUFFER_FORMAT::NV_ENC_BUFFER_FORMAT_NV12;

    enc_status = enc_fn.nvEncCreateInputBuffer.unwrap()(encoder, &mut enc_input_buffer_params);

    /*
     * Register the frames received from NvFBC for use with NvEncodeAPI.
     */

    cu_res = nv_cuda.cuMemAllocPitch_v2(&mut buffer, &mut 3440, frame_size.w as usize,
        frame_size.h as usize, 16);
    if cu_res != cuda::cudaError_enum::CUDA_SUCCESS {
        panic!("Unable to initialize CUDA buffer. Result: {}", cu_res as u32);
    }

    let mut enc_register_params = std::mem::MaybeUninit::<enc::NV_ENC_REGISTER_RESOURCE>::zeroed().assume_init();
    enc_register_params.version = nvenc_struct_version(3);
    enc_register_params.resourceType = enc::NV_ENC_INPUT_RESOURCE_TYPE::NV_ENC_INPUT_RESOURCE_TYPE_CUDADEVICEPTR;
    enc_register_params.resourceToRegister = buffer as *mut std::ffi::c_void;
    enc_register_params.width = frame_size.w;
    enc_register_params.height = frame_size.h;
    enc_register_params.pitch = 0;
    enc_register_params.bufferFormat = enc::NV_ENC_BUFFER_FORMAT::NV_ENC_BUFFER_FORMAT_NV12;

    enc_status = enc_fn.nvEncRegisterResource.unwrap()(encoder, &mut enc_register_params);
    if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
        panic!("Failed to initialize the CUDA device encoder resource. Status = {}", enc_status as u32);
    }

    /*
     * Create a bitstream buffer to hold the output
     */
    let mut enc_bitstream_buffer_params = std::mem::MaybeUninit::<enc::NV_ENC_CREATE_BITSTREAM_BUFFER>::zeroed().assume_init();
    enc_bitstream_buffer_params.version = nvenc_struct_version(1);

    enc_status = enc_fn.nvEncCreateBitstreamBuffer.unwrap()(encoder, &mut enc_bitstream_buffer_params);
    if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
        panic!("Failed to create a bitstream buffer. Status = {}", enc_status as u32);
    }

    let mut enc_output_buffer = enc_bitstream_buffer_params.bitstreamBuffer;

    // let mut file = File::create("out.h264").unwrap();
    let mut file = stdio::fopen(std::ffi::CString::new("out.h264").unwrap().as_ptr(), std::ffi::CString::new("wb").unwrap().as_ptr());

    /*
     * Pre-fill mapping information
     */
    let mut enc_map_params = std::mem::MaybeUninit::<enc::NV_ENC_MAP_INPUT_RESOURCE>::zeroed().assume_init();
    enc_map_params.version = nvenc_struct_version(4);

    let mut enc_params = std::mem::MaybeUninit::<enc::NV_ENC_PIC_PARAMS>::zeroed().assume_init();
    enc_params.version = nvenc_struct_version(4) | ( 1<<31 );
    enc_params.inputWidth = frame_size.w;
    enc_params.inputHeight = frame_size.h;
    enc_params.inputPitch = frame_size.w;
    enc_params.pictureStruct = enc::NV_ENC_PIC_STRUCT::NV_ENC_PIC_STRUCT_FRAME;
    enc_params.outputBitstream = enc_output_buffer;

    /*
     * We are now ready to start grabbing frames.
     */
    let mut index = 0;
    loop {
        let mut fbc_grab_params = std::mem::MaybeUninit::<fbc::NVFBC_TOCUDA_GRAB_FRAME_PARAMS>::zeroed().assume_init();
        let mut fbc_frame_info = std::mem::MaybeUninit::<fbc::NVFBC_FRAME_GRAB_INFO>::zeroed().assume_init();
    
        fbc_grab_params.dwVersion = nvfbc_struct_version::<fbc::NVFBC_TOCUDA_GRAB_FRAME_PARAMS>(2);
        fbc_grab_params.dwFlags = fbc::NVFBC_TOCUDA_FLAGS::NVFBC_TOCUDA_GRAB_FLAGS_NOFLAGS as u32;
        fbc_grab_params.pFrameGrabInfo = &mut fbc_frame_info;
        // fbc_grab_params.pCUDADeviceBuffer = std::ptr::addr_of_mut!(buffer) as *mut std::ffi::c_void;
        fbc_grab_params.pCUDADeviceBuffer = &mut buffer as *mut _ as *mut std::ffi::c_void;
        fbc_grab_params.dwTimeoutMs = 0;
    
        /*
         * Capture a frame.
         */
        fbc_status = cap_fn.nvFBCToCudaGrabFrame.unwrap()(fbc_handle, &mut fbc_grab_params);
        if fbc_status == fbc::NVFBCSTATUS::NVFBC_ERR_MUST_RECREATE {
            println!("Capture session must be recreated!");
            break;
        } else if fbc_status != fbc::NVFBCSTATUS::NVFBC_SUCCESS {
            let error = std::ffi::CStr::from_ptr(cap_fn.nvFBCGetLastErrorStr.unwrap()(fbc_handle)).to_str().unwrap();
            panic!("{}", error);
        }

        /*
         * Map the frame for use by the encoder.
         */
        enc_map_params.registeredResource = enc_register_params.registeredResource;

        enc_status = enc_fn.nvEncMapInputResource.unwrap()(encoder, &mut enc_map_params);
        if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
            panic!("Failed to map resource. Status: {}", enc_status as u32); // this is might cause memory leaks and other nasty problems *shrugs*
        }

        let mut input_buffer = enc_map_params.mappedResource;
        enc_params.inputBuffer = input_buffer;
        enc_params.bufferFmt = enc_map_params.mappedBufferFmt;

        enc_params.inputTimeStamp = index;
        enc_params.frameIdx = index as u32;

        /*
         * Encode the frame.
         */
        enc_status = enc_fn.nvEncEncodePicture.unwrap()(encoder, &mut enc_params);
        if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
            println!("Failed to encode frame. Status: {}", enc_status as u32);
        } else {
            let mut enc_lock_params = std::mem::MaybeUninit::<enc::NV_ENC_LOCK_BITSTREAM>::zeroed().assume_init();
            enc_lock_params.version = nvenc_struct_version(1);
            enc_lock_params.outputBitstream = enc_output_buffer;

            enc_status = enc_fn.nvEncLockBitstream.unwrap()(encoder, &mut enc_lock_params);
            if enc_status == enc::NVENCSTATUS::NV_ENC_SUCCESS {
                let buffer_size = enc_lock_params.bitstreamSizeInBytes;
                if buffer_size == 0 {
                    panic!("Failed to obtain bitstream buffer.");
                }
        
                stdio::fwrite(enc_lock_params.bitstreamBufferPtr, 1, buffer_size as u64, file);

                enc_status = enc_fn.nvEncUnlockBitstream.unwrap()(encoder, enc_output_buffer);
                if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
                    println!("Failed to unlock bitstream buffer. Status: {}", enc_status as u32);
                }    
            } else {
                println!("Failed to lock bitstream buffer. Status: {}", enc_status as u32);
            }
        }

        /*
         * Unmap the frame.
         */
        enc_status = enc_fn.nvEncUnmapInputResource.unwrap()(encoder, input_buffer);
        if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
            panic!("Failed to unmap input resource. Status: {}", enc_status as u32);
        }

        index = index.wrapping_add(1);
        if index == 500 {
            enc_params.version = nvenc_struct_version(4) | ( 1<<31 );
            enc_params.encodePicFlags = enc::NV_ENC_PIC_FLAGS::NV_ENC_PIC_FLAG_EOS as u32;

            enc_status = enc_fn.nvEncEncodePicture.unwrap()(encoder, &mut enc_params);
            if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
                println!("Failed to flush the encoder. Status: {}", enc_status as u32);
            }  

            enc_status = enc_fn.nvEncDestroyBitstreamBuffer.unwrap()(encoder, enc_output_buffer);
            if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
                println!("Failed to destroy output buffer. Status: {}", enc_status as u32);
            }

            stdio::fclose(file);

            enc_status = enc_fn.nvEncDestroyEncoder.unwrap()(encoder);
            if enc_status != enc::NVENCSTATUS::NV_ENC_SUCCESS {
                println!("Failed to destroy encoder. Status: {}", enc_status as u32);
            }
            break;
        }
    }

    println!("I ran succesfully tf???");
}
}

fn nvfbc_version() -> u32 {
    fbc::NVFBC_VERSION_MINOR | (fbc::NVFBC_VERSION_MAJOR << 8)
}

fn nvfbc_struct_version<T>(ver: u32) -> u32 {
    std::mem::size_of::<T>() as u32 | ((ver) << 16) | (nvfbc_version() << 24)
}

fn nvenc_version() -> u32 {
    enc::NVENCAPI_MAJOR_VERSION | (enc::NVENCAPI_MINOR_VERSION << 24)
}

fn nvenc_struct_version(ver: u32) -> u32 {
    nvenc_version() | ((ver)<<16) | (0x7 << 28)
}
