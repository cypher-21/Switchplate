use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::warn;

const SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Chime {
    Request,
    Question,
    Completion,
    Approve,
    Reject,
}

impl Chime {
    fn samples(self) -> Vec<u8> {
        // Each bell uses a fundamental, a quieter overtone, and a decaying envelope.
        let notes: &[(f32, f32, f32, f32)] = match self {
            Self::Request => &[(587.33, 0.0, 0.30, 0.24), (880.0, 0.09, 0.43, 0.28)],
            Self::Question => &[(659.25, 0.0, 0.28, 0.24), (987.77, 0.16, 0.38, 0.22)],
            Self::Completion => &[
                (523.25, 0.0, 0.43, 0.21),
                (659.25, 0.10, 0.47, 0.21),
                (783.99, 0.20, 0.53, 0.23),
            ],
            Self::Approve => &[(659.25, 0.0, 0.26, 0.18), (880.0, 0.04, 0.35, 0.20)],
            Self::Reject => &[(320.0, 0.0, 0.11, 0.16), (240.0, 0.03, 0.12, 0.16)],
        };
        let duration = notes.iter().map(|note| note.1 + note.2).fold(0.0, f32::max);

        (0..(duration * SAMPLE_RATE as f32).ceil() as u32)
            .flat_map(|sample| {
                let elapsed = sample as f32 / SAMPLE_RATE as f32;
                let amplitude: f32 = notes
                    .iter()
                    .map(|&(frequency, start, length, volume)| {
                        let time = elapsed - start;
                        if !(0.0..length).contains(&time) {
                            return 0.0;
                        }
                        let phase = std::f32::consts::TAU * frequency * time;
                        let attack = (time / 0.008).min(1.0);
                        let decay = (-5.0 * time / length).exp();
                        let release = ((length - time) / 0.02).min(1.0);
                        (phase.sin() + (phase * 2.76).sin() * 0.18)
                            * attack
                            * decay
                            * release
                            * volume
                    })
                    .sum();
                let pcm = (amplitude.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
                pcm.to_le_bytes()
            })
            .collect()
    }
}

/// Play on the desktop audio server independently of webview visibility and focus.
pub fn play(chime: Chime) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = play_checked(chime).await {
            warn!("{error}");
        }
    });
}

pub async fn play_checked(chime: Chime) -> Result<(), String> {
    let samples = chime.samples();
    let rate = SAMPLE_RATE.to_string();
    let players: &[(&str, &[&str])] = &[
        (
            "pw-play",
            &[
                "--raw",
                "--format=s16",
                "--rate",
                &rate,
                "--channels=1",
                "--media-role=Notification",
                "-",
            ],
        ),
        (
            "paplay",
            &[
                "--raw",
                "--format=s16le",
                "--rate",
                &rate,
                "--channels=1",
                "--client-name=Switchplate",
            ],
        ),
        (
            "aplay",
            &["-q", "-t", "raw", "-f", "S16_LE", "-r", &rate, "-c", "1"],
        ),
    ];

    for &(program, args) in players {
        let Ok(mut child) = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
        else {
            continue;
        };
        let playback = async {
            let mut input = child.stdin.take().expect("player stdin is piped");
            let written = input.write_all(&samples).await;
            drop(input);
            Ok::<_, std::io::Error>(child.wait().await?.success() && written.is_ok())
        };
        if matches!(
            tokio::time::timeout(Duration::from_secs(3), playback).await,
            Ok(Ok(true))
        ) {
            return Ok(());
        }
    }
    Err("Could not play a Switchplate chime. Check the desktop audio output or install pw-play or paplay.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_response_have_distinct_non_silent_pcm_samples() {
        let request = Chime::Request.samples();
        let completion = Chime::Completion.samples();
        assert!(completion.len() > request.len());
        assert!((49_900..50_000).contains(&request.len()));
        for chime in [request, completion] {
            let peak = chime
                .chunks_exact(2)
                .map(|p| i16::from_le_bytes([p[0], p[1]]).unsigned_abs())
                .max()
                .unwrap();
            assert!((4000..16000).contains(&peak));
        }
    }
}
