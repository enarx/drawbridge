// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use rcgen::{generate_simple_self_signed, Certificate};
use reqwest::{Client, Identity};
use std::time::Duration;

pub struct TestHelper {
    pub server_cert: Certificate,
    pub server_pem: Vec<u8>,
    pub server_private_pem: Vec<u8>,
    pub client_cert: Certificate,
    pub client_cert_private_pem: String,
    pub ca: Certificate,
    pub ca_pem: Vec<u8>,
    pub client_cert_pem: String,
    pub client_cert_with_private_key: Vec<u8>,
    pub other_ca: Certificate,
    pub client_other_cert: Certificate,
    pub client_other_cert_pem: String,
    pub client_other_cert_with_private_key: Vec<u8>,
}

impl Default for TestHelper {
    fn default() -> Self {
        let server_cert = generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
        let server_pem = server_cert.serialize_pem().unwrap().as_bytes().to_vec();
        let server_private_pem = server_cert.serialize_private_key_pem().as_bytes().to_vec();
        let client_cert = generate_simple_self_signed(vec![]).unwrap();
        let client_cert_private_pem = client_cert.serialize_private_key_pem();

        let ca = generate_simple_self_signed(vec![]).unwrap();
        let ca_pem = ca.serialize_pem().unwrap().as_bytes().to_vec();
        let client_cert_pem = client_cert.serialize_pem_with_signer(&ca).unwrap();
        let client_cert_with_private_key =
            format!("{}\n{}", client_cert_pem, client_cert_private_pem)
                .as_bytes()
                .to_vec();

        // other ca is a certificate root not in use by the server
        let other_ca = generate_simple_self_signed(vec![]).unwrap();
        let client_other_cert = generate_simple_self_signed(vec![]).unwrap();
        let client_other_cert_pem = client_other_cert
            .serialize_pem_with_signer(&other_ca)
            .unwrap();
        let client_other_cert_with_private_key =
            format!("{}\n{}", client_other_cert_pem, client_cert_private_pem)
                .as_bytes()
                .to_vec();

        Self {
            server_cert,
            server_pem,
            server_private_pem,
            client_cert,
            client_cert_private_pem,
            ca,
            ca_pem,
            client_cert_pem,
            client_cert_with_private_key,
            other_ca,
            client_other_cert,
            client_other_cert_pem,
            client_other_cert_with_private_key,
        }
    }
}

impl TestHelper {
    pub fn unauthenticated_client(&self) -> Client {
        create_client(&self.server_pem, None)
    }

    pub fn certificate_auth_client(&self) -> Client {
        create_client(
            &self.server_pem,
            Some(Identity::from_pem(&self.client_cert_with_private_key).unwrap()),
        )
    }

    pub fn invalid_certificate_auth_client(&self) -> Client {
        create_client(
            &self.server_pem,
            Some(Identity::from_pem(&self.client_other_cert_with_private_key).unwrap()),
        )
    }
}

fn create_client(server_cert: &[u8], identity: Option<Identity>) -> Client {
    let mut client = Client::builder()
        .timeout(Duration::new(1, 0))
        .add_root_certificate(reqwest::Certificate::from_pem(server_cert).unwrap())
        // TODO: there seems to be some kind of error when this is disabled
        .danger_accept_invalid_certs(true);

    if let Some(identity) = identity {
        client = client.identity(identity);
    }

    client.build().unwrap()
}
