use std::sync::Mutex;

#[derive(serde::Serialize)]
pub struct GetStateResponse {
    #[serde(flatten)]
    pub state: crate::state::AppState,
    pub state_recovered: bool,
}

#[tauri::command]
pub fn get_state(
    state: tauri::State<Mutex<crate::state::AppState>>,
    state_recovered: tauri::State<bool>,
) -> GetStateResponse {
    GetStateResponse {
        state: crate::state::recover_lock(state.inner()).clone(),
        state_recovered: *state_recovered.inner(),
    }
}

#[tauri::command]
pub fn save_preference(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
    key: String,
    value: String,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    let changed = match key.as_str() {
        "font_size" => {
            let next = value.parse::<f32>().ok();
            if s.preferences.font_size == next {
                false
            } else {
                s.preferences.font_size = next;
                true
            }
        }
        "theme" => {
            let next = Some(value);
            if s.preferences.theme == next {
                false
            } else {
                s.preferences.theme = next;
                true
            }
        }
        "font_family" => {
            let next = Some(value);
            if s.preferences.font_family == next {
                false
            } else {
                s.preferences.font_family = next;
                true
            }
        }
        "line_height" => {
            let next = value.parse::<f32>().ok();
            if s.preferences.line_height == next {
                false
            } else {
                s.preferences.line_height = next;
                true
            }
        }
        "text_align" => {
            let next = Some(value);
            if s.preferences.text_align == next {
                false
            } else {
                s.preferences.text_align = next;
                true
            }
        }
        "reader_mode" => {
            let next = match value.as_str() {
                "scroll" => Some(String::from("scroll")),
                _ => Some(String::from("paginated")),
            };
            if s.preferences.reader_mode == next {
                false
            } else {
                s.preferences.reader_mode = next;
                true
            }
        }
        _ => false,
    };
    if !changed {
        return Ok(());
    }
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}

#[tauri::command]
pub fn clear_last_opened(
    state: tauri::State<Mutex<crate::state::AppState>>,
    data_dir: tauri::State<std::path::PathBuf>,
) -> Result<(), String> {
    let mut s = crate::state::recover_lock(state.inner());
    if s.last_opened.is_none() {
        return Ok(());
    }
    s.last_opened = None;
    let data_dir = data_dir.inner().clone();
    crate::state::save_state(&data_dir, &s)
}
