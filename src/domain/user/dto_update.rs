#[derive(Debug, Clone)]
pub struct UpdateUserDto {
    pub username: Option<String>,
    pub email: Option<String>,
}
