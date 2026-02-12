pub fn validate_pagination(page: Option<u64>, limit: Option<u64>) -> (u64, u64) {
    let page = page.unwrap_or(1).max(1);
    let limit = limit.unwrap_or(20).clamp(1, 100);
    (page, limit)
}
