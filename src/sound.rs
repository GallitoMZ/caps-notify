use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

pub enum SoundTheme {
    ModernChime,
    KeyClick,
    WindowsDefault,
    None,
}

impl SoundTheme {
    pub fn from_str(s: &str) -> Self {
        match s {
            "ModernChime" => SoundTheme::ModernChime,
            "KeyClick" => SoundTheme::KeyClick,
            "WindowsDefault" => SoundTheme::WindowsDefault,
            _ => SoundTheme::None,
        }
    }
}

pub fn play(theme_name: &str, enabled: bool) {
    match SoundTheme::from_str(theme_name) {
        SoundTheme::ModernChime => {
            let wav = generate_chime_wav(enabled);
            play_wav_memory(&wav);
        }
        SoundTheme::KeyClick => {
            let wav = generate_click_wav(enabled);
            play_wav_memory(&wav);
        }
        SoundTheme::WindowsDefault => unsafe {
            let _ = PlaySoundW(
                windows::core::w!("SystemNotification"),
                None,
                SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
            );
        },
        SoundTheme::None => {}
    }
}

fn play_wav_memory(wav: &[u8]) {
    unsafe {
        // When SND_MEMORY is used, pszsound is a pointer to the in-memory WAV file
        let _ = PlaySoundW(
            PCWSTR(wav.as_ptr() as *const u16),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

fn generate_wav_header(data_len: usize) -> Vec<u8> {
    let sample_rate: u32 = 44100;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * (channels as u32) * ((bits_per_sample / 8) as u32);
    let block_align = channels * (bits_per_sample / 8);
    let file_size: u32 = 36 + (data_len as u32);

    let mut h = Vec::with_capacity(44);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&file_size.to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
    h.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 for PCM)
    h.extend_from_slice(&channels.to_le_bytes());
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&byte_rate.to_le_bytes());
    h.extend_from_slice(&block_align.to_le_bytes());
    h.extend_from_slice(&bits_per_sample.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&(data_len as u32).to_le_bytes());
    h
}

/// Generates a pleasant, soft, elegant harmonic chime (80ms)
fn generate_chime_wav(on: bool) -> Vec<u8> {
    let sample_rate = 44100.0;
    let duration_sec = 0.085;
    let total_samples = (sample_rate * duration_sec) as usize;
    let data_len = total_samples * 2;

    let mut wav = generate_wav_header(data_len);

    let (f1, f2) = if on {
        (880.0, 1760.0) // A5 + A6 (bright soft ding for ON)
    } else {
        (587.33, 1174.66) // D5 + D6 (gentle lower tone for OFF)
    };

    let pi = std::f32::consts::PI;
    let attack_samples = (sample_rate * 0.006) as usize; // 6ms soft attack

    for i in 0..total_samples {
        let t = i as f32 / sample_rate;
        // Cosine attack, exponential decay
        let attack = if i < attack_samples {
            0.5 * (1.0 - (pi * i as f32 / attack_samples as f32).cos())
        } else {
            1.0
        };
        let decay = (-t * 35.0).exp();
        let amp = attack * decay;

        let sample1 = (2.0 * pi * f1 * t).sin();
        let sample2 = 0.28 * (2.0 * pi * f2 * t).sin();
        let val = (sample1 + sample2) * amp * 0.65;

        let val_i16 = (val.clamp(-1.0, 1.0) * 32767.0) as i16;
        wav.extend_from_slice(&val_i16.to_le_bytes());
    }

    wav
}

/// Generates a crisp, subtle mechanical switch actuation click (25ms)
fn generate_click_wav(on: bool) -> Vec<u8> {
    let sample_rate = 44100.0;
    let duration_sec = 0.028;
    let total_samples = (sample_rate * duration_sec) as usize;
    let data_len = total_samples * 2;

    let mut wav = generate_wav_header(data_len);
    let freq = if on { 1350.0 } else { 920.0 };
    let pi = std::f32::consts::PI;

    for i in 0..total_samples {
        let t = i as f32 / sample_rate;
        let decay = (-t * 120.0).exp();
        let sample = (2.0 * pi * freq * t).sin() * decay * 0.6;
        let val_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        wav.extend_from_slice(&val_i16.to_le_bytes());
    }

    wav
}
