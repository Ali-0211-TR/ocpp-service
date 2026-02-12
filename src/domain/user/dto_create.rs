use super::UserRole;

#[derive(Debug, Clone)]
pub struct CreateUserDto {
    pub username: String,
    pub email: String,
    pub role: Option<UserRole>,
    pub password: String,
}
