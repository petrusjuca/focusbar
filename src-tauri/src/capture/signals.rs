//! Sinais nativos do SO pro motor de presença: áudio tocando e tela bloqueada.
//! Idle é o sinal PRIMÁRIO; estes refinam (Passivo = parado mas com áudio;
//! Ausente imediato quando a tela trava). Cada plataforma num `mod imp` isolado.

/// Tem áudio TOCANDO agora? (separa "assistindo um vídeo" de "AFK").
pub fn audio_active() -> bool {
    imp::audio_active()
}

/// A sessão está BLOQUEADA (tela de bloqueio)? Ausência limpa e certa.
pub fn locked() -> bool {
    imp::locked()
}

// ───────────────────────── macOS (testável no dev) ─────────────────────────
#[cfg(target_os = "macos")]
mod imp {
    use std::os::raw::c_void;

    #[repr(C)]
    struct AudioObjectPropertyAddress {
        selector: u32,
        scope: u32,
        element: u32,
    }

    // FourCC dos seletores do CoreAudio.
    const SYSTEM_OBJECT: u32 = 1; // kAudioObjectSystemObject
    const DEFAULT_OUTPUT: u32 = 0x644F_7574; // 'dOut'
    const RUNNING_SOMEWHERE: u32 = 0x676F_6E65; // 'gone' = IsRunningSomewhere
    const SCOPE_GLOBAL: u32 = 0x676C_6F62; // 'glob'
    const ELEMENT_MAIN: u32 = 0;

    #[link(name = "CoreAudio", kind = "framework")]
    extern "C" {
        fn AudioObjectGetPropertyData(
            in_object: u32,
            in_address: *const AudioObjectPropertyAddress,
            in_qualifier_size: u32,
            in_qualifier: *const c_void,
            io_data_size: *mut u32,
            out_data: *mut c_void,
        ) -> i32; // OSStatus (0 = ok)
    }

    fn get_u32(object: u32, selector: u32) -> Option<u32> {
        let addr = AudioObjectPropertyAddress {
            selector,
            scope: SCOPE_GLOBAL,
            element: ELEMENT_MAIN,
        };
        let mut out: u32 = 0;
        let mut size: u32 = 4;
        let st = unsafe {
            AudioObjectGetPropertyData(
                object,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                &mut out as *mut u32 as *mut c_void,
            )
        };
        if st == 0 {
            Some(out)
        } else {
            None
        }
    }

    pub fn audio_active() -> bool {
        // 1) descobre o dispositivo de saída padrão; 2) pergunta se ele está tocando.
        match get_u32(SYSTEM_OBJECT, DEFAULT_OUTPUT) {
            Some(dev) if dev != 0 => get_u32(dev, RUNNING_SOMEWHERE).unwrap_or(0) != 0,
            _ => false,
        }
    }

    pub fn locked() -> bool {
        use core_foundation::base::TCFType;
        use core_foundation::string::CFString;

        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGSessionCopyCurrentDictionary() -> *const c_void;
        }
        extern "C" {
            fn CFDictionaryGetValue(d: *const c_void, key: *const c_void) -> *const c_void;
            fn CFBooleanGetValue(b: *const c_void) -> u8;
            fn CFRelease(cf: *const c_void);
        }
        unsafe {
            let dict = CGSessionCopyCurrentDictionary();
            if dict.is_null() {
                return false;
            }
            let key = CFString::new("CGSSessionScreenIsLocked");
            let val = CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void);
            let locked = !val.is_null() && CFBooleanGetValue(val) != 0;
            CFRelease(dict);
            locked
        }
    }
}

// ───────────────────────── Windows (validado no CI) ─────────────────────────
#[cfg(target_os = "windows")]
mod imp {
    pub fn audio_active() -> bool {
        use windows::core::Interface;
        use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
        use windows::Win32::Media::Audio::{
            eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
        };
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
        };
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                    Ok(e) => e,
                    Err(_) => return false,
                };
            let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                Ok(d) => d,
                Err(_) => return false,
            };
            let meter: IAudioMeterInformation = match device.Activate(CLSCTX_ALL, None) {
                Ok(m) => m,
                Err(_) => return false,
            };
            match meter.GetPeakValue() {
                Ok(peak) => peak > 0.01,
                Err(_) => false,
            }
        }
    }

    pub fn locked() -> bool {
        use windows::Win32::System::RemoteDesktop::{
            WTSFreeMemory, WTSQuerySessionInformationW, WTSSessionInfoEx, WTSINFOEXW,
            WTS_CURRENT_SERVER_HANDLE, WTS_CURRENT_SESSION,
        };
        const WTS_SESSIONSTATE_LOCK: u32 = 0;
        unsafe {
            let mut buf: *mut u16 = std::ptr::null_mut();
            let mut bytes: u32 = 0;
            let ok = WTSQuerySessionInformationW(
                Some(WTS_CURRENT_SERVER_HANDLE),
                WTS_CURRENT_SESSION,
                WTSSessionInfoEx,
                &mut buf as *mut _ as *mut windows::core::PWSTR,
                &mut bytes,
            );
            if ok.is_err() || buf.is_null() {
                return false;
            }
            let info = &*(buf as *const WTSINFOEXW);
            let flags = info.Data.WTSInfoExLevel1.SessionFlags;
            WTSFreeMemory(buf as *mut core::ffi::c_void);
            flags == WTS_SESSIONSTATE_LOCK as i32
        }
    }
}

// ───────────────────────── outros SOs ─────────────────────────
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod imp {
    pub fn audio_active() -> bool {
        false
    }
    pub fn locked() -> bool {
        false
    }
}
