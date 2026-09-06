use bitfiddle_engine::{render::render_offline, validate::validate_document};
use serde::Serialize;

const MAX_OFFLINE_SECONDS: f64 = 3_600.0;

#[derive(Debug, Clone, Serialize)]
pub struct ValidationResponse {
    pub canonical_yaml: String,
    pub module_count: usize,
    pub wire_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineRenderResponse {
    pub sample_rate: u32,
    pub frame_count: usize,
    pub interleaved_stereo: Vec<f32>,
}

pub fn validate_rack(yaml: String) -> Result<ValidationResponse, String> {
    let document = validate_document(&yaml).map_err(|error| error.to_string())?;
    let canonical_yaml = document.to_yaml().map_err(|error| error.to_string())?;
    Ok(ValidationResponse {
        module_count: document.modules.len(),
        wire_count: document.wires.len(),
        canonical_yaml,
    })
}

pub fn render_rack_offline(yaml: String, seconds: f64) -> Result<OfflineRenderResponse, String> {
    if !seconds.is_finite() || seconds <= 0.0 || seconds > MAX_OFFLINE_SECONDS {
        return Err(format!(
            "seconds must be finite and between 0 and {MAX_OFFLINE_SECONDS}"
        ));
    }
    let document = validate_document(&yaml).map_err(|error| error.to_string())?;
    let sample_rate = document.engine.sample_rate;
    let frames = render_offline(document, seconds).map_err(|error| error.to_string())?;
    let frame_count = frames.len();
    let interleaved_stereo = frames
        .into_iter()
        .flat_map(|frame| frame.into_iter())
        .collect();
    Ok(OfflineRenderResponse {
        sample_rate,
        frame_count,
        interleaved_stereo,
    })
}

#[cfg(feature = "desktop")]
mod desktop {
    use super::{OfflineRenderResponse, ValidationResponse};

    #[tauri::command]
    fn validate_rack(yaml: String) -> Result<ValidationResponse, String> {
        super::validate_rack(yaml)
    }

    #[tauri::command]
    fn render_rack_offline(yaml: String, seconds: f64) -> Result<OfflineRenderResponse, String> {
        super::render_rack_offline(yaml, seconds)
    }

    #[cfg_attr(mobile, tauri::mobile_entry_point)]
    pub fn run() {
        tauri::Builder::default()
            .invoke_handler(tauri::generate_handler![validate_rack, render_rack_offline])
            .run(tauri::generate_context!())
            .expect("failed to run bitfiddle");
    }
}

#[cfg(feature = "desktop")]
pub use desktop::run;

#[cfg(test)]
mod tests {
    use super::*;

    const SINE_RACK: &str = include_str!("../../../fixtures/sine.bitfiddle.yaml");

    #[test]
    fn validates_and_canonicalizes_rack() {
        let response = validate_rack(SINE_RACK.to_owned()).expect("fixture validates");
        assert_eq!(response.module_count, 2);
        assert_eq!(response.wire_count, 1);
        assert!(response.canonical_yaml.ends_with('\n'));
    }

    #[test]
    fn renders_bounded_stereo_frames() {
        let response = render_rack_offline(SINE_RACK.to_owned(), 0.001).expect("fixture renders");
        assert_eq!(response.sample_rate, 48_000);
        assert_eq!(response.frame_count, 48);
        assert_eq!(response.interleaved_stereo.len(), 96);
    }

    #[test]
    fn rejects_invalid_render_duration_before_rendering() {
        let error = render_rack_offline(SINE_RACK.to_owned(), f64::INFINITY)
            .expect_err("invalid duration rejected");
        assert!(error.contains("seconds must be finite"));
    }
}
