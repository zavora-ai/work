//! Presenting live.
//!
//! Speaking a slide is one thing; presenting is another. A presenter who cannot be stopped is a
//! recording, and a presenter who cannot be asked a question is a video. This holds a session open
//! for as long as the deck is being shown, so what is said can be cut off the moment the presenter
//! moves on, and a question can be put and answered in the middle of it.
//!
//! One session per deck being presented, because there is one presenter. Moving to the next slide
//! interrupts what is being said rather than queueing behind it — a presenter who talks over the
//! next slide is worse than one who stops mid-sentence.

use std::sync::Arc;

use adk_realtime::{RealtimeConfig, ServerEvent};
use tokio::sync::Mutex;

/// What came back from the presenter, in the terms the interface needs.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Heard {
    /// A piece of speech, as base64 samples to play.
    Sound { base64: String },
    /// What is being said, in words, so it can be shown as well as heard.
    Words { text: String },
    /// The presenter has finished this piece.
    Finished,
    /// It could not go on, and this is why.
    Trouble { detail: String },
    /// Something the interface has no use for — the session opening, a response beginning.
    ///
    /// Reported rather than swallowed, because "nothing I care about" and "the session has ended"
    /// are different facts and a caller that cannot tell them apart stops listening at the first
    /// housekeeping message. Which is exactly what happened.
    Nothing,
}

/// The presenter, for as long as the deck is up.
pub struct Live {
    session: adk_realtime::BoxedSession,
}

/// The one session there is. A second presenter talking over the first is not a feature.
static PRESENTER: Mutex<Option<Arc<Live>>> = Mutex::const_new(None);

impl Live {
    /// Open a session, told what it is presenting and how to behave.
    pub async fn open(voice: &str, about: &str) -> Result<Self, String> {
        let key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| "Work Studio has not been given a way to speak yet".to_string())?;

        let config = RealtimeConfig {
            voice: Some(voice.to_string()),
            // Both, because the words are shown on screen while they are spoken: a presenter
            // reading from the screen and an audience reading the same words should agree.
            modalities: Some(vec!["audio".to_string(), "text".to_string()]),
            instruction: Some(format!(
                "You are presenting a deck to an audience, out loud. Say what you are given, in \
                 the words you are given, as a person presenting would say them. Do not add \
                 commentary, do not describe the slide, and do not mention that you are an \
                 assistant. If asked a question, answer it briefly from what the deck says and \
                 say so when the deck does not say. The deck is about: {about}"
            )),
            ..RealtimeConfig::default()
        };

        let model = adk_realtime::openai::OpenAIRealtimeModel::with_default_model(key);
        let session = adk_realtime::RealtimeModel::connect(&model, config)
            .await
            .map_err(|error| error.to_string())?;

        Ok(Self { session })
    }

    /// Say this, stopping whatever was being said.
    pub async fn say(&self, words: &str) -> Result<(), String> {
        // The interruption comes first. Moving to the next slide while the last one is still being
        // spoken is the ordinary case, not the exception.
        let _ = self.session.interrupt().await;
        // Said as an instruction, not as conversation. Sent plainly, the session takes the slide
        // as something to reply to — asked to present "seventy per cent of children lack access",
        // it answered "wow, that's a huge and important issue", which is not presenting.
        self.session
            .send_text(&format!(
                "Present this slide by saying exactly the following words, and nothing else. Do \
                 not react to them, do not introduce them, do not add anything: {words}"
            ))
            .await
            .map_err(|error| error.to_string())?;
        self.session
            .create_response()
            .await
            .map_err(|error| error.to_string())
    }

    /// Someone in the room has a question.
    ///
    /// Interrupts first, because a question asked over the presenter is a question asked while it
    /// is talking — that is what a question in a room is. The answer comes from the deck, and the
    /// session was told to say so when the deck does not say.
    pub async fn asked(&self, question: &str) -> Result<(), String> {
        let _ = self.session.interrupt().await;
        self.session
            .send_text(&format!(
                "Someone in the audience asks: {question}\n\nAnswer briefly, from what this deck \
                 says. If the deck does not say, say that it does not."
            ))
            .await
            .map_err(|error| error.to_string())?;
        self.session
            .create_response()
            .await
            .map_err(|error| error.to_string())
    }

    /// A question asked out loud, as recorded samples.
    ///
    /// Sent as it arrived. The session hears it and decides for itself when the asker has finished,
    /// which is a judgement about a room and not one to make here.
    pub async fn heard_question(&self, samples_base64: &str) -> Result<(), String> {
        let _ = self.session.interrupt().await;
        self.session
            .send_audio_base64(samples_base64)
            .await
            .map_err(|error| error.to_string())?;
        self.session
            .commit_audio()
            .await
            .map_err(|error| error.to_string())?;
        self.session
            .create_response()
            .await
            .map_err(|error| error.to_string())
    }

    /// Stop talking, now.
    pub async fn hush(&self) -> Result<(), String> {
        self.session
            .interrupt()
            .await
            .map_err(|error| error.to_string())
    }

    /// The next thing to come back, in the terms the interface needs.
    pub async fn next(&self) -> Option<Heard> {
        match self.session.next_event().await? {
            Ok(ServerEvent::AudioDelta { delta, .. }) => Some(Heard::Sound {
                // Encoded because it crosses a channel that carries text. The samples themselves
                // are what the session sent; nothing is resampled or reinterpreted on the way.
                base64: base64_of(&delta),
            }),
            Ok(ServerEvent::TranscriptDelta { delta, .. }) => Some(Heard::Words { text: delta }),
            Ok(ServerEvent::ResponseDone { .. }) => Some(Heard::Finished),
            Ok(_) => Some(Heard::Nothing),
            Err(error) => Some(Heard::Trouble {
                detail: error.to_string(),
            }),
        }
    }
}

/// Bytes as base64, for a channel that carries text.
fn base64_of(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let packed = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(packed >> 18) as usize & 63] as char);
        out.push(ALPHABET[(packed >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(packed >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[packed as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Open the presenter, replacing any that was already talking.
pub async fn begin(voice: &str, about: &str) -> Result<(), String> {
    let live = Arc::new(Live::open(voice, about).await?);
    let mut held = PRESENTER.lock().await;
    *held = Some(live);
    Ok(())
}

/// The presenter, if one is up.
pub async fn presenter() -> Option<Arc<Live>> {
    PRESENTER.lock().await.clone()
}

/// Close the session. Nothing keeps talking after the deck comes down.
pub async fn end() {
    let taken = PRESENTER.lock().await.take();
    if let Some(live) = taken {
        let _ = live.hush().await;
    }
}

#[cfg(test)]
mod tests {
    use super::base64_of;

    /// The encoding has to be the one every decoder expects, padding included.
    #[test]
    fn bytes_encode_the_way_a_decoder_reads_them() {
        assert_eq!(base64_of(b""), "");
        assert_eq!(base64_of(b"f"), "Zg==");
        assert_eq!(base64_of(b"fo"), "Zm8=");
        assert_eq!(base64_of(b"foo"), "Zm9v");
        assert_eq!(base64_of(b"foob"), "Zm9vYg==");
        assert_eq!(base64_of(&[0x00, 0xff, 0x80]), "AP+A");
    }
}
