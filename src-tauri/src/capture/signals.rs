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

// ───────────────────────── Windows ─────────────────────────
// Paridade com o Mac: áudio via WASAPI (IAudioMeterInformation.GetPeakValue) e
// tela bloqueada via "input desktop" (quando travado, o desktop de entrada vira
// o seguro/Winlogon, não o "Default"). Tudo por polling, sem loop de mensagens —
// casa com o sampler que já chama isso 1x por tick.
#[cfg(target_os = "windows")]
mod imp {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::StationsAndDesktops::{
        CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, DESKTOP_READOBJECTS,
        DF_ALLOWOTHERACCOUNTHOOK, UOI_NAME,
    };

    /// Tem som saindo agora? Pega o medidor do dispositivo de saída padrão e olha
    /// o pico. Limiar baixo: separa "vídeo tocando" de silêncio. Qualquer falha de
    /// COM/dispositivo → false (idle continua sendo o sinal primário).
    pub fn audio_active() -> bool {
        unsafe {
            // COM em multithread; se já inicializado em outro modo, segue mesmo assim.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) {
                    Ok(e) => e,
                    Err(_) => return false,
                };
            let device = match enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                Ok(d) => d,
                Err(_) => return false, // sem dispositivo de saída
            };
            // Activate usa PROPVARIANT internamente (None = sem parâmetros).
            let meter: IAudioMeterInformation = match device.Activate(CLSCTX_ALL, None) {
                Ok(m) => m,
                Err(_) => return false,
            };
            meter.GetPeakValue().unwrap_or(0.0) > 0.01
        }
    }

    /// A tela está bloqueada? Abre o "input desktop": travado → o desktop de
    /// entrada é o seguro (não "Default"), ou nem dá pra abrir. Polling puro.
    pub fn locked() -> bool {
        unsafe {
            let hdesk = match OpenInputDesktop(
                DF_ALLOWOTHERACCOUNTHOOK,
                false,
                DESKTOP_READOBJECTS,
            ) {
                Ok(h) => h,
                Err(_) => return true, // não conseguiu abrir → provavelmente travado/seguro
            };

            let mut buf = [0u16; 256];
            let mut needed: u32 = 0;
            // GetUserObjectInformationW recebe HANDLE genérico; HDESK é o mesmo ponteiro.
            let got = GetUserObjectInformationW(
                HANDLE(hdesk.0),
                UOI_NAME,
                Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
                (buf.len() * 2) as u32,
                Some(&mut needed),
            );
            let _ = CloseDesktop(hdesk);

            if got.is_err() {
                return false; // na dúvida, NÃO marca ausente (evita falso positivo)
            }
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let name = String::from_utf16_lossy(&buf[..len]);
            // "Default" = desktop normal (desbloqueado). Qualquer outro = travado.
            !name.eq_ignore_ascii_case("Default")
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
