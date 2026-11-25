//! Bounded worker-owned credentials; never sent to the admission owner or logs.
use zeroize::Zeroizing;
pub(crate) struct Credentials {
    user: Zeroizing<[u8; 64]>,
    password: Zeroizing<[u8; 128]>,
    user_len: usize,
    password_len: usize,
}
impl Credentials {
    pub fn new(user: &str, password: &str) -> Option<Self> {
        if user.is_empty()
            || user.len() > 64
            || !user
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.@-".contains(&c))
            || password.is_empty()
            || password.len() > 128
            || !password.bytes().all(|b| (32..=126).contains(&b))
        {
            return None;
        }
        let mut value = Self {
            user: Zeroizing::new([0; 64]),
            password: Zeroizing::new([0; 128]),
            user_len: user.len(),
            password_len: password.len(),
        };
        value.user[..user.len()].copy_from_slice(user.as_bytes());
        value.password[..password.len()].copy_from_slice(password.as_bytes());
        Some(value)
    }
    pub fn user(&self) -> &[u8] {
        &self.user[..self.user_len]
    }
    pub fn password(&self) -> &[u8] {
        &self.password[..self.password_len]
    }
}
