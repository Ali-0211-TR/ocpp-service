#[derive(Debug, Clone)]
pub struct UserChangePasswordDto {
    pub username: String,
    pub current_password: String,
    pub new_password: String,
}
