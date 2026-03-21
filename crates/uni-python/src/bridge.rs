//! Embedded Python bridge library (`uni_bridge.py`).

use std::path::{Path, PathBuf};

/// Embedded Python bridge library.
/// Extracted to `{base_dir}/lib/uni_bridge.py` at runtime.
const BRIDGE_PY: &str = include_str!("../python/lib/uni_bridge.py");

/// Ensure `uni_bridge.py` is available in the lib directory.
/// Creates `{base_dir}/lib/uni_bridge.py` if missing or if content changed.
/// Returns path to the lib directory.
pub async fn ensure_bridge(base_dir: &Path) -> Result<PathBuf, uni_common::UniError> {
    let lib_dir = base_dir.join("lib");
    tokio::fs::create_dir_all(&lib_dir)
        .await
        .map_err(uni_common::UniError::Io)?;

    let bridge_path = lib_dir.join("uni_bridge.py");

    let needs_write = if bridge_path.exists() {
        let existing = tokio::fs::read_to_string(&bridge_path)
            .await
            .unwrap_or_default();
        existing != BRIDGE_PY
    } else {
        true
    };

    if needs_write {
        tokio::fs::write(&bridge_path, BRIDGE_PY)
            .await
            .map_err(uni_common::UniError::Io)?;
        log::info!("Extracted uni_bridge.py to {:?}", bridge_path);
    }

    Ok(lib_dir)
}
