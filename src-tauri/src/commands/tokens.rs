use tiktoken_rs::o200k_base;

#[tauri::command]
pub fn count_tokens(text: String) -> Result<usize, String> {
    let bpe = o200k_base().map_err(|e| e.to_string())?;
    Ok(bpe.encode_with_special_tokens(&text).len())
}
