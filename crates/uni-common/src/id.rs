/// Generate a new UUID v4 string.
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
