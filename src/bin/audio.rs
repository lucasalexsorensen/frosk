use std::{
    mem::size_of,
    ptr, slice,
    sync::{Arc, Condvar, Mutex},
};

use windows::{
    core::{implement, IUnknown, Interface, HRESULT, PCSTR, PROPVARIANT},
    Win32::{
        Foundation::WAIT_OBJECT_0,
        Media::{
            Audio::{
                ActivateAudioInterfaceAsync, IActivateAudioInterfaceAsyncOperation,
                IActivateAudioInterfaceCompletionHandler,
                IActivateAudioInterfaceCompletionHandler_Impl, IAudioCaptureClient, IAudioClient,
                AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                AUDCLNT_STREAMFLAGS_LOOPBACK, AUDIOCLIENT_ACTIVATION_PARAMS,
                AUDIOCLIENT_ACTIVATION_PARAMS_0, AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
                AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS,
                PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
                VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
                WAVEFORMATEXTENSIBLE_0,
            },
            KernelStreaming::WAVE_FORMAT_EXTENSIBLE,
            Multimedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
        },
        System::{
            Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
            Threading::{CreateEventA, WaitForSingleObject},
            Variant::VT_BLOB,
        },
    },
};

#[implement(IActivateAudioInterfaceCompletionHandler)]
struct Handler(Arc<(Mutex<bool>, Condvar)>);

impl Handler {
    pub fn new(object: Arc<(Mutex<bool>, Condvar)>) -> Handler {
        Handler(object)
    }
}

impl IActivateAudioInterfaceCompletionHandler_Impl for Handler {
    fn ActivateCompleted(
        &self,
        _activateoperation: Option<&IActivateAudioInterfaceAsyncOperation>,
    ) -> windows::core::Result<()> {
        let (lock, cvar) = &*self.0;
        let mut completed = lock.lock().unwrap();
        *completed = true;
        drop(completed);
        cvar.notify_one();
        Ok(())
    }
}

fn main() -> windows::core::Result<()> {
    let process_id = 36000;

    unsafe {
        // Initialize COM
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

        // Create audio client
        let audio_client_activation_params = AUDIOCLIENT_ACTIVATION_PARAMS {
            ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
                ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                    TargetProcessId: process_id,
                    ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
                },
            },
        };

        let raw_prop = windows_core::imp::PROPVARIANT {
            Anonymous: windows_core::imp::PROPVARIANT_0 {
                Anonymous: windows_core::imp::PROPVARIANT_0_0 {
                    vt: VT_BLOB.0,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: windows_core::imp::PROPVARIANT_0_0_0 {
                        blob: windows_core::imp::BLOB {
                            cbSize: size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
                            pBlobData: &audio_client_activation_params as *const _ as *mut _,
                        },
                    },
                },
            },
        };

        let activation_prop = PROPVARIANT::from_raw(raw_prop);
        let activation_params = Some(&activation_prop as *const _);
        let riid = IAudioClient::IID;

        // Create completion handler
        let setup = Arc::new((Mutex::new(false), Condvar::new()));
        let callback: IActivateAudioInterfaceCompletionHandler = Handler::new(setup.clone()).into();

        // Activate audio interface
        let operation = ActivateAudioInterfaceAsync(
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
            &riid,
            activation_params,
            &callback,
        )?;

        // Wait for completion
        let (lock, cvar) = &*setup;
        let mut completed = lock.lock().unwrap();
        while !*completed {
            completed = cvar.wait(completed).unwrap();
        }
        drop(completed);

        // Get audio client and result
        let mut audio_client: Option<IUnknown> = Default::default();
        let mut result: HRESULT = Default::default();
        operation.GetActivateResult(&mut result, &mut audio_client)?;

        // Ensure successful activation
        result.ok()?;
        let audio_client: IAudioClient = audio_client.unwrap().cast()?;

        // Audio client arguments
        let block_align = 2 * 32 / 8;
        let byte_rate = 44100 * block_align;

        let extensible = WAVEFORMATEXTENSIBLE {
            Format: WAVEFORMATEX {
                wFormatTag: WAVE_FORMAT_EXTENSIBLE as u16,
                nChannels: 2,
                nSamplesPerSec: 44100,
                nAvgBytesPerSec: byte_rate,
                nBlockAlign: block_align as u16,
                wBitsPerSample: 32,
                cbSize: (size_of::<WAVEFORMATEXTENSIBLE>() - size_of::<WAVEFORMATEX>()) as u16,
            },
            Samples: WAVEFORMATEXTENSIBLE_0 {
                wValidBitsPerSample: 32,
            },
            SubFormat: KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
            dwChannelMask: 0x1 | 0x2,
        };

        let stream_flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_LOOPBACK;

        // Initialise audio client
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            stream_flags,
            200000,
            0,
            &extensible.Format,
            None,
        )?;

        // Get capture client
        let capture_client = audio_client.GetService::<IAudioCaptureClient>()?;

        // Set event handle
        let h_event = CreateEventA(None, false, false, PCSTR::null())?;
        audio_client.SetEventHandle(h_event)?;
        audio_client.Start()?;

        println!("Starting main loop");

        // Main loop
        loop {
            let frames_available = capture_client.GetNextPacketSize()?;
            if frames_available < 1 {
                continue;
            }

            // Get pointer to buffer
            let mut buffer_ptr = ptr::null_mut();
            let mut nbr_frames_returned = 0;
            let mut flags = 0;
            capture_client.GetBuffer(
                &mut buffer_ptr,
                &mut nbr_frames_returned,
                &mut flags,
                None,
                None,
            )?;

            // Fill buffer
            let len_in_bytes = nbr_frames_returned as usize * block_align as usize;
            let bufferslice = slice::from_raw_parts(buffer_ptr, len_in_bytes);

            // Look for a non silent sample
            let mut non_zero = false;
            for element in bufferslice.iter() {
                if *element != 0 {
                    non_zero = true;
                }
            }

            if non_zero {
                println!("Non-zero sample found");
            }

            // Release buffer
            if nbr_frames_returned > 0 {
                capture_client.ReleaseBuffer(nbr_frames_returned).unwrap();
            }

            // Read from device to queue
            let retval = WaitForSingleObject(h_event, 100000);
            if retval.0 != WAIT_OBJECT_0.0 {
                panic!("AHHHHH");
            }
        }
    }
}