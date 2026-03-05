use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const TARGET_SAMPLE_RATE: u32 = 16000;
const TARGET_CHANNELS: u16 = 1;

pub struct AudioRecorder {
    buffer: Arc<Mutex<Vec<i16>>>,
    is_recording: Arc<AtomicBool>,
    thread_handle: Mutex<Option<JoinHandle<()>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            thread_handle: Mutex::new(None),
        }
    }

    pub fn start(&self) -> Result<(), String> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err("Already recording".to_string());
        }

        {
            let mut buf = self.buffer.lock().map_err(|e| e.to_string())?;
            buf.clear();
        }

        self.is_recording.store(true, Ordering::SeqCst);

        let buffer = self.buffer.clone();
        let is_recording = self.is_recording.clone();

        let handle = std::thread::spawn(move || {
            if let Err(e) = run_recording(buffer, is_recording) {
                log::error!("Recording thread error: {}", e);
            }
        });

        let mut th = self.thread_handle.lock().map_err(|e| e.to_string())?;
        *th = Some(handle);

        Ok(())
    }

    pub fn stop(&self) -> Result<Vec<i16>, String> {
        self.is_recording.store(false, Ordering::SeqCst);

        let handle = {
            let mut th = self.thread_handle.lock().map_err(|e| e.to_string())?;
            th.take()
        };

        if let Some(h) = handle {
            h.join().map_err(|_| "Recording thread panicked".to_string())?;
        }

        let mut buf = self.buffer.lock().map_err(|e| e.to_string())?;
        let samples = std::mem::take(&mut *buf);
        Ok(samples)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }
}

fn run_recording(
    buffer: Arc<Mutex<Vec<i16>>>,
    is_recording: Arc<AtomicBool>,
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;

    log::info!("Recording device: {}", device.name().unwrap_or_default());

    let config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get default input config: {}", e))?;

    let device_sample_rate = config.sample_rate().0;
    let device_channels = config.channels();

    log::info!(
        "Device config: {}Hz, {} channels, {:?}",
        device_sample_rate,
        device_channels,
        config.sample_format()
    );

    let buf_clone = buffer.clone();
    let rec_flag = is_recording.clone();

    let process_samples = move |raw_i16: Vec<i16>| {
        if !rec_flag.load(Ordering::SeqCst) {
            return;
        }

        let mono = to_mono(&raw_i16, device_channels as usize);
        let resampled = if device_sample_rate != TARGET_SAMPLE_RATE {
            resample(&mono, device_sample_rate, TARGET_SAMPLE_RATE)
        } else {
            mono
        };

        if let Ok(mut buf) = buf_clone.lock() {
            buf.extend_from_slice(&resampled);
        }
    };

    let err_fn = |err: cpal::StreamError| {
        log::error!("Audio stream error: {}", err);
    };

    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let process_f32 = {
        let ps = process_samples.clone();
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let i16_data: Vec<i16> = data.iter().map(|&s| f32_to_i16(s)).collect();
            ps(i16_data);
        }
    };

    let process_i16 = {
        let ps = process_samples.clone();
        move |data: &[i16], _: &cpal::InputCallbackInfo| {
            ps(data.to_vec());
        }
    };

    let process_u16 = {
        let ps = process_samples;
        move |data: &[u16], _: &cpal::InputCallbackInfo| {
            let i16_data: Vec<i16> = data.iter().map(|&s| (s as i32 - 32768) as i16).collect();
            ps(i16_data);
        }
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device
            .build_input_stream(&stream_config, process_f32, err_fn, None)
            .map_err(|e| format!("Failed to build F32 stream: {}", e))?,
        cpal::SampleFormat::I16 => device
            .build_input_stream(&stream_config, process_i16, err_fn, None)
            .map_err(|e| format!("Failed to build I16 stream: {}", e))?,
        cpal::SampleFormat::U16 => device
            .build_input_stream(&stream_config, process_u16, err_fn, None)
            .map_err(|e| format!("Failed to build U16 stream: {}", e))?,
        other => return Err(format!("Unsupported sample format: {:?}", other)),
    };

    stream
        .play()
        .map_err(|e| format!("Failed to start stream: {}", e))?;

    while is_recording.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    drop(stream);
    Ok(())
}

fn to_mono(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
            (sum / channels as i32) as i16
        })
        .collect()
}

fn resample(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;
    let mut result = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        if idx + 1 < samples.len() {
            let frac = src_idx - idx as f64;
            let a = samples[idx] as f64;
            let b = samples[idx + 1] as f64;
            result.push((a + (b - a) * frac) as i16);
        } else if idx < samples.len() {
            result.push(samples[idx]);
        }
    }
    result
}

fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * 32767.0) as i16
}
