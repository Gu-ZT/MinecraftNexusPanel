use std::fs;
use std::fs::File;
#[cfg(unix)]
use std::fs::Permissions;
use std::io;
use std::io::BufReader;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use nexus_config::CoreConfig;
use nexus_protocol::certificate_sha256;
use rcgen::CertifiedKey;
use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use rustls_pemfile::certs;
use rustls_pemfile::private_key;
use tokio_rustls::TlsAcceptor;

use crate::CoreTlsIdentityError;

const CORE_ALPN_PROTOCOL: &[u8] = b"mcnp/1";
const DEFAULT_CERTIFICATE_FILE_NAME: &str = "core-cert.pem";
const DEFAULT_PRIVATE_KEY_FILE_NAME: &str = "core-key.pem";
const TLS_DIRECTORY_NAME: &str = "tls";

pub(crate) struct CoreTlsIdentity {
    acceptor: TlsAcceptor,
    certificate_sha256: String,
}

impl CoreTlsIdentity {
    pub(crate) fn load_or_create(config: &CoreConfig) -> Result<Self, CoreTlsIdentityError> {
        let (certificate_path, private_key_path) = identity_paths(config)?;
        if config.tls_certificate_path().is_none() {
            ensure_default_identity(&certificate_path, &private_key_path)?;
        }

        let certificates = load_certificates(&certificate_path)?;
        let private_key = load_private_key(&private_key_path)?;
        let certificate_sha256 = certificates
            .first()
            .map(|certificate| certificate_sha256(certificate.as_ref()))
            .ok_or_else(|| CoreTlsIdentityError::EmptyCertificateChain {
                path: certificate_path.clone(),
            })?;
        let mut server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)
            .map_err(CoreTlsIdentityError::InvalidIdentity)?;
        server_config.alpn_protocols = vec![CORE_ALPN_PROTOCOL.to_vec()];

        Ok(Self {
            acceptor: TlsAcceptor::from(Arc::new(server_config)),
            certificate_sha256,
        })
    }

    pub(crate) fn acceptor(&self) -> TlsAcceptor {
        self.acceptor.clone()
    }

    pub(crate) fn certificate_sha256(&self) -> &str {
        &self.certificate_sha256
    }
}

fn identity_paths(config: &CoreConfig) -> Result<(PathBuf, PathBuf), CoreTlsIdentityError> {
    match (config.tls_certificate_path(), config.tls_private_key_path()) {
        (Some(certificate_path), Some(private_key_path)) => Ok((
            certificate_path.to_path_buf(),
            private_key_path.to_path_buf(),
        )),
        (None, None) => {
            let tls_directory = config.data_directory().join(TLS_DIRECTORY_NAME);
            Ok((
                tls_directory.join(DEFAULT_CERTIFICATE_FILE_NAME),
                tls_directory.join(DEFAULT_PRIVATE_KEY_FILE_NAME),
            ))
        }
        (Some(_), None) | (None, Some(_)) => {
            Err(CoreTlsIdentityError::IncompleteConfiguredIdentity)
        }
    }
}

fn ensure_default_identity(
    certificate_path: &Path,
    private_key_path: &Path,
) -> Result<(), CoreTlsIdentityError> {
    match (certificate_path.exists(), private_key_path.exists()) {
        (true, true) => return Ok(()),
        (true, false) | (false, true) => {
            return Err(CoreTlsIdentityError::IncompleteDefaultIdentity {
                certificate_path: certificate_path.to_path_buf(),
                private_key_path: private_key_path.to_path_buf(),
            });
        }
        (false, false) => {}
    }

    let directory =
        certificate_path
            .parent()
            .ok_or_else(|| CoreTlsIdentityError::CreateDirectory {
                path: certificate_path.to_path_buf(),
                source: invalid_identity_directory(),
            })?;
    fs::create_dir_all(directory).map_err(|source| CoreTlsIdentityError::CreateDirectory {
        path: directory.to_path_buf(),
        source,
    })?;
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(["localhost".to_owned()])
        .map_err(CoreTlsIdentityError::Generate)?;
    fs::write(private_key_path, key_pair.serialize_pem()).map_err(|source| {
        CoreTlsIdentityError::WritePrivateKey {
            path: private_key_path.to_path_buf(),
            source,
        }
    })?;
    restrict_private_key(private_key_path)?;
    fs::write(certificate_path, cert.pem()).map_err(|source| {
        CoreTlsIdentityError::WriteCertificate {
            path: certificate_path.to_path_buf(),
            source,
        }
    })
}

fn load_certificates(path: &Path) -> Result<Vec<CertificateDer<'static>>, CoreTlsIdentityError> {
    let file = File::open(path).map_err(|source| CoreTlsIdentityError::ReadCertificate {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::new(file);
    let certificates = certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| CoreTlsIdentityError::ReadCertificate {
            path: path.to_path_buf(),
            source,
        })?;
    if certificates.is_empty() {
        return Err(CoreTlsIdentityError::EmptyCertificateChain {
            path: path.to_path_buf(),
        });
    }

    Ok(certificates)
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, CoreTlsIdentityError> {
    let file = File::open(path).map_err(|source| CoreTlsIdentityError::ReadPrivateKey {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::new(file);

    private_key(&mut reader)
        .map_err(|source| CoreTlsIdentityError::ReadPrivateKey {
            path: path.to_path_buf(),
            source,
        })?
        .ok_or_else(|| CoreTlsIdentityError::MissingPrivateKey {
            path: path.to_path_buf(),
        })
}

#[cfg(unix)]
fn restrict_private_key(path: &Path) -> Result<(), CoreTlsIdentityError> {
    fs::set_permissions(path, Permissions::from_mode(0o600)).map_err(|source| {
        CoreTlsIdentityError::RestrictPrivateKey {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn restrict_private_key(_path: &Path) -> Result<(), CoreTlsIdentityError> {
    Ok(())
}

fn invalid_identity_directory() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "TLS certificate path has no parent directory",
    )
}
