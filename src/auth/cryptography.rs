use zbus::interface;

use crate::flatpak::service::FlatpakService;

pub struct Cryptography {
    pub service: FlatpakService,
}

impl Cryptography {
    pub fn new(service: FlatpakService) -> Self {
        Cryptography { service }
    }
}

#[interface(name = "inc.kora.auth.Cryptography")]
impl Cryptography {
    async fn get_public_key(&mut self) -> (String, String) {
        let key_type: &str = "RSA-SHA256";
        (key_type.to_string(), self.service.get_public_key())
    }
}
