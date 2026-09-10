#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeckoCookie {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) host: String,
    pub(crate) path: String,
    pub(crate) expiry: i64,
    pub(crate) is_secure: bool,
    pub(crate) is_http_only: bool,
    pub(crate) same_site: i64,
    pub(crate) origin_attributes: String,
}
