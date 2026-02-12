use super::UserRole;

#[derive(Debug, Clone)]
pub struct GetUserDto {
    pub search: Option<String>,
    pub role: Option<UserRole>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort_by: Option<String>,
}
