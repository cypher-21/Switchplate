let audioCtx: AudioContext | null = null;

function getAudioContext(): AudioContext | null {
  try {
    if (!audioCtx) {
      const AudioContextClass =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (AudioContextClass) {
        audioCtx = new AudioContextClass();
      }
    }
    if (audioCtx && audioCtx.state === "suspended") {
      audioCtx.resume();
    }
    return audioCtx;
  } catch {
    return null;
  }
}

/**
 * Play a double-chime when a new permission request arrives.
 * Clean, warm sine tones: 587.33Hz (D5) -> 880Hz (A5).
 */
export function playRequestChime() {
  const ctx = getAudioContext();
  if (!ctx) return;

  try {
    const now = ctx.currentTime;

    // First tone (587 Hz - D5)
    const osc1 = ctx.createOscillator();
    const gain1 = ctx.createGain();
    osc1.type = "sine";
    osc1.frequency.setValueAtTime(587.33, now);
    gain1.gain.setValueAtTime(0.0001, now);
    gain1.gain.exponentialRampToValueAtTime(0.18, now + 0.025);
    gain1.gain.exponentialRampToValueAtTime(0.0001, now + 0.15);
    osc1.connect(gain1);
    gain1.connect(ctx.destination);
    osc1.start(now);
    osc1.stop(now + 0.16);

    // Second tone (880 Hz - A5)
    const osc2 = ctx.createOscillator();
    const gain2 = ctx.createGain();
    osc2.type = "sine";
    osc2.frequency.setValueAtTime(880, now + 0.1);
    gain2.gain.setValueAtTime(0.0001, now + 0.1);
    gain2.gain.exponentialRampToValueAtTime(0.2, now + 0.13);
    gain2.gain.exponentialRampToValueAtTime(0.0001, now + 0.36);
    osc2.connect(gain2);
    gain2.connect(ctx.destination);
    osc2.start(now + 0.1);
    osc2.stop(now + 0.38);
  } catch (err) {
    console.warn("Audio playback error:", err);
  }
}

/**
 * Play an affirmative confirmation chord when an action is approved / allowed.
 * Harmonic chord: 523.25Hz (C5) + 659.25Hz (E5) + 783.99Hz (G5).
 */
export function playApproveChime() {
  const ctx = getAudioContext();
  if (!ctx) return;

  try {
    const now = ctx.currentTime;
    const freqs = [523.25, 659.25, 783.99];

    freqs.forEach((freq, idx) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "triangle";
      osc.frequency.setValueAtTime(freq, now + idx * 0.035);
      gain.gain.setValueAtTime(0.0001, now + idx * 0.035);
      gain.gain.exponentialRampToValueAtTime(0.12, now + idx * 0.035 + 0.025);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + idx * 0.035 + 0.32);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(now + idx * 0.035);
      osc.stop(now + idx * 0.035 + 0.34);
    });
  } catch (err) {
    console.warn("Audio playback error:", err);
  }
}

/**
 * Play a subtle low mechanical dismiss tone when an action is rejected.
 * 320Hz -> 200Hz downward sweep.
 */
export function playRejectChime() {
  const ctx = getAudioContext();
  if (!ctx) return;

  try {
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "sine";
    osc.frequency.setValueAtTime(320, now);
    osc.frequency.exponentialRampToValueAtTime(200, now + 0.16);
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(0.14, now + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.18);
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start(now);
    osc.stop(now + 0.19);
  } catch (err) {
    console.warn("Audio playback error:", err);
  }
}
