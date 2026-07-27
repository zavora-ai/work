//! Saying it out loud.
//!
//! The presenter's voice. A talk goes in, sound comes back — as a playable file rather than raw
//! samples, because the interface's job is to play it and not to know what a sample rate is.
//!
//! The credential is the one the rest of Work Studio uses, and the sound is made on this machine and
//! handed straight to the window: nothing about the User's deck is left anywhere it can be found
//! again.

use adk_audio::{CloudTtsConfig, OpenAiTts, TtsProvider, TtsRequest};

/// Sound, ready to play, and how long it lasts.
pub struct Spoken {
    /// A complete WAV file. The interface plays it; it does not have to assemble anything.
    pub wav: Vec<u8>,
    pub milliseconds: u32,
}

/// Say these words in this voice.
pub async fn speak(words: &str, voice: &str) -> Result<Spoken, String> {
    if words.trim().is_empty() {
        return Err("there is nothing to say".to_string());
    }

    let key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "Work Studio has not been given a way to speak yet".to_string())?;

    let provider = OpenAiTts::new(CloudTtsConfig::new(key));
    let frame = provider
        .synthesize(&TtsRequest {
            text: words.to_string(),
            voice: voice.to_string(),
            ..TtsRequest::default()
        })
        .await
        .map_err(|error| error.to_string())?;

    Ok(Spoken {
        milliseconds: frame.duration_ms,
        wav: wav_of(&frame.data, frame.sample_rate, frame.channels),
    })
}

/// Raw samples wrapped as a WAV file.
///
/// The provider hands back signed 16-bit samples and nothing else. Forty-four bytes of header is the
/// difference between something the interface can play and something it would have to decode.
fn wav_of(samples: &[u8], sample_rate: u32, channels: u8) -> Vec<u8> {
    let channels = channels.max(1) as u16;
    let bits = 16u16;
    let byte_rate = sample_rate * channels as u32 * (bits / 8) as u32;
    let block_align = channels * (bits / 8);

    let mut wav = Vec::with_capacity(44 + samples.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + samples.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // uncompressed
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    wav.extend_from_slice(samples);
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wrapper_is_a_wav_file_a_player_will_accept() {
        let samples = vec![0u8; 3200];
        let wav = wav_of(&samples, 24_000, 1);

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + samples.len());

        // The sizes in the header have to describe the file, or a player rejects it.
        let riff = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        assert_eq!(riff as usize, wav.len() - 8);
        let data = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(data as usize, samples.len());
        let rate = u32::from_le_bytes(wav[24..28].try_into().unwrap());
        assert_eq!(rate, 24_000);
    }

    #[tokio::test]
    async fn nothing_to_say_is_refused_rather_than_sent() {
        assert!(speak("   ", "alloy").await.is_err());
    }
}
