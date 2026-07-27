/**
 * Playing speech as it arrives.
 *
 * A live presenter sends sound in pieces while it is still deciding what to say, so waiting for a
 * whole sentence would put the voice a sentence behind the slide. Each piece is queued to play
 * directly after the one before it, which is what makes it sound like someone talking rather than a
 * series of clips.
 *
 * Stopping means stopping: when the presenter moves on, everything queued is dropped rather than
 * played out. A voice that finishes the previous slide over the new one is worse than one that stops
 * mid-word.
 */

/// What the live session sends: signed 16-bit samples at 24kHz, one channel.
const SAMPLE_RATE = 24_000;

export class SpeechPlayer {
  private context: AudioContext | undefined;
  /// When the next piece should start. Ahead of the clock while a queue is building.
  private nextAt = 0;
  private playing: AudioBufferSourceNode[] = [];
  private onQuiet: (() => void) | undefined;

  /** Play a piece of speech, after whatever is already queued. */
  add(base64: string): void {
    const samples = decode(base64);
    if (samples.length === 0) return;

    // Created on first use, because a context made before the User has asked for anything is a
    // context the window may refuse to start.
    this.context ??= new AudioContext({ sampleRate: SAMPLE_RATE });
    const context = this.context;
    void context.resume();

    const buffer = context.createBuffer(1, samples.length, SAMPLE_RATE);
    const channel = buffer.getChannelData(0);
    for (let at = 0; at < samples.length; at += 1) {
      // Signed 16-bit to the -1..1 the audio engine works in.
      channel[at] = samples[at]! / 32768;
    }

    const source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(context.destination);

    // A small cushion the first time, so the first piece is not chasing the clock and clipping its
    // own beginning.
    const startAt = Math.max(this.nextAt, context.currentTime + 0.06);
    source.start(startAt);
    this.nextAt = startAt + buffer.duration;

    this.playing.push(source);
    source.onended = () => {
      this.playing = this.playing.filter((one) => one !== source);
      if (this.playing.length === 0) this.onQuiet?.();
    };
  }

  /** Stop, dropping whatever was queued. */
  stop(): void {
    for (const source of this.playing) {
      try {
        source.stop();
      } catch {
        // Already finished; nothing to stop.
      }
    }
    this.playing = [];
    this.nextAt = 0;
  }

  /** Whether anything is being said now. */
  get speaking(): boolean {
    return this.playing.length > 0;
  }

  /** Told when the last queued piece has finished. */
  whenQuiet(listener: () => void): void {
    this.onQuiet = listener;
  }

  /** Let the audio device go. */
  close(): void {
    this.stop();
    void this.context?.close();
    this.context = undefined;
  }
}

/** Base64 to signed 16-bit samples. */
function decode(base64: string): Int16Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let at = 0; at < binary.length; at += 1) bytes[at] = binary.charCodeAt(at);
  // An odd byte cannot be half a sample; the remainder is left rather than guessed at.
  return new Int16Array(bytes.buffer, 0, Math.floor(bytes.length / 2));
}
