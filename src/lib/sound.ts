// SWITCHPLATE — Studio Acoustic Bell & Chime Synthesis (Web Audio API)
// Uses additive acoustic partials with gentle exponential decay and lowpass damping.
// Clean, elegant, zero-latency, no external audio files required.

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
 * Creates an acoustic bell partial with natural envelope and lowpass damping.
 */
function playBellPartial(
  ctx: AudioContext,
  freq: number,
  gainLevel: number,
  startTime: number,
  duration: number
) {
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  const filter = ctx.createBiquadFilter();

  // Gentle lowpass filter to remove harsh edge
  filter.type = "lowpass";
  filter.frequency.setValueAtTime(3200, startTime);
  filter.Q.setValueAtTime(1.2, startTime);

  osc.type = "sine";
  osc.frequency.setValueAtTime(freq, startTime);

  // Acoustic strike envelope: 4ms strike attack, natural exponential decay
  gain.gain.setValueAtTime(0.00001, startTime);
  gain.gain.exponentialRampToValueAtTime(gainLevel, startTime + 0.005);
  gain.gain.exponentialRampToValueAtTime(0.00001, startTime + duration);

  osc.connect(filter);
  filter.connect(gain);
  gain.connect(ctx.destination);

  osc.start(startTime);
  osc.stop(startTime + duration + 0.05);
}

/**
 * Play soft acoustic bell chime for incoming approval requests or when opening island.
 * Timbre: Pure crystal chime (D5 -> A5 with 2nd partials).
 */
export function playRequestChime() {
  const ctx = getAudioContext();
  if (!ctx) return;

  try {
    const t0 = ctx.currentTime;

    // First bell: D5 (587.33 Hz) + harmonic (1620 Hz)
    playBellPartial(ctx, 587.33, 0.16, t0, 0.28);
    playBellPartial(ctx, 1621.0, 0.035, t0, 0.18);

    // Second bell: A5 (880.00 Hz) + harmonic (2428 Hz)
    const t1 = t0 + 0.09;
    playBellPartial(ctx, 880.0, 0.19, t1, 0.42);
    playBellPartial(ctx, 2428.8, 0.045, t1, 0.25);
  } catch (err) {
    console.warn("Audio chime error:", err);
  }
}

/**
 * Play a rich, warm completion chime when OpenCode/AI finishes generating.
 * Timbre: Warm acoustic resolve (F5 + C6 + A6 harmonic).
 */
export function playCompletionChime() {
  const ctx = getAudioContext();
  if (!ctx) return;

  try {
    const t0 = ctx.currentTime;

    // Triad strike: F5 (698.46 Hz) + C6 (1046.50 Hz) + E6 (1318.51 Hz)
    playBellPartial(ctx, 698.46, 0.12, t0, 0.48);
    playBellPartial(ctx, 1046.5, 0.14, t0 + 0.03, 0.55);
    playBellPartial(ctx, 1318.51, 0.08, t0 + 0.06, 0.6);
  } catch (err) {
    console.warn("Audio chime error:", err);
  }
}

/**
 * Play an affirmative chord for immediate user approval.
 */
export function playApproveChime() {
  const ctx = getAudioContext();
  if (!ctx) return;

  try {
    const t0 = ctx.currentTime;
    playBellPartial(ctx, 659.25, 0.14, t0, 0.26);
    playBellPartial(ctx, 880.0, 0.16, t0 + 0.04, 0.35);
  } catch (err) {
    console.warn("Audio chime error:", err);
  }
}

/**
 * Play a quiet mechanical tactile click for rejection / dismiss.
 */
export function playRejectChime() {
  const ctx = getAudioContext();
  if (!ctx) return;

  try {
    const t0 = ctx.currentTime;
    playBellPartial(ctx, 320.0, 0.12, t0, 0.09);
    playBellPartial(ctx, 240.0, 0.08, t0 + 0.02, 0.08);
  } catch (err) {
    console.warn("Audio chime error:", err);
  }
}
