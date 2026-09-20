#[cfg(unix)]
use std::path::PathBuf;

/// Transport abstraction for daemon listeners.
/// Unix targets `~/.algo/algo.sock` with 0600; Windows stub returns error (P4).
#[async_trait::async_trait]
pub trait Transport: Send + Sync + Sized {
    async fn listen(path: &str) -> std::io::Result<Self>;
    async fn accept(&self) -> std::io::Result<TransportStream>;
    fn socket_path(&self) -> &str;
}

/// Stream returned by `Transport::accept`.
pub enum TransportStream {
    #[cfg(unix)]
    Unix(tokio::net::UnixStream),
    #[cfg(windows)]
    NamedPipe(tokio::net::windows::named_pipe::NamedPipeServer),
    /// Fallback stub used when platform does not support the requested transport.
    #[allow(dead_code)]
    Stub,
}

impl TransportStream {
    /// Split into read/write halves for line-based NDJSON.
    /// Only Unix is fully implemented; Windows stub returns error on use.
    #[cfg(unix)]
    pub fn into_unix(self) -> Option<tokio::net::UnixStream> {
        match self {
            Self::Unix(s) => Some(s),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    #[cfg(not(unix))]
    pub fn into_unix(self) -> Option<()> {
        None
    }
}

/// Unix domain socket transport (production on Unix).
pub struct UnixTransport {
    path: String,
    #[cfg(unix)]
    listener: tokio::net::UnixListener,
}

#[async_trait::async_trait]
impl Transport for UnixTransport {
    async fn listen(path: &str) -> std::io::Result<Self> {
        #[cfg(unix)]
        {
            let p = PathBuf::from(path);
            if let Some(parent) = p.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            // Remove stale socket
            let _ = tokio::fs::remove_file(path).await;
            let listener = tokio::net::UnixListener::bind(path)?;
            // Enforce 0600 perms
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(path, perms)?;
            }
            Ok(Self {
                path: path.to_string(),
                listener,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "UnixListener not supported on this platform; use NamedPipe (P4)",
            ))
        }
    }

    async fn accept(&self) -> std::io::Result<TransportStream> {
        #[cfg(unix)]
        {
            let (stream, _) = self.listener.accept().await?;
            Ok(TransportStream::Unix(stream))
        }
        #[cfg(not(unix))]
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Unix accept not supported on this platform",
            ))
        }
    }

    fn socket_path(&self) -> &str {
        &self.path
    }
}

/// Named pipe transport stub for Windows (P4).
/// On non-Windows it always errors; on Windows it is still a stub until P4.
#[derive(Debug)]
pub struct NamedPipeTransport {
    path: String,
}

#[async_trait::async_trait]
impl Transport for NamedPipeTransport {
    async fn listen(path: &str) -> std::io::Result<Self> {
        #[cfg(windows)]
        {
            // P4 placeholder: real NamedPipe server would be created here.
            // For now, return unsupported to keep the trait compileable and to prove
            // fail-safe: callers handle the error and map to ask/fallback.
            let _ = path;
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "NamedPipe transport not yet implemented (P4 stub)",
            ))
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "NamedPipe not supported on this platform (P4 Windows only)",
            ))
        }
    }

    async fn accept(&self) -> std::io::Result<TransportStream> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "NamedPipe accept stub (P4)",
        ))
    }

    fn socket_path(&self) -> &str {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn named_pipe_stub_errors() {
        let res = NamedPipeTransport::listen("\\\\.\\pipe\\algo-test").await;
        assert!(res.is_err());
        let kind = res.unwrap_err().kind();
        assert_eq!(kind, std::io::ErrorKind::Unsupported);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_transport_binds_and_perms() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("algo.sock");
        let path = sock.to_string_lossy().to_string();
        let t = UnixTransport::listen(&path).await.expect("bind");
        assert_eq!(t.socket_path(), path);
        // Check perms 0600
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "socket perms must be 0600");
        // Accept should not error immediately (we test that accept blocks; just check we can drop)
        drop(t);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_transport_accept() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("algo-accept.sock");
        let path = sock.to_string_lossy().to_string();
        let t = UnixTransport::listen(&path).await.unwrap();
        let path2 = path.clone();
        let server = tokio::spawn(async move {
            let stream = t.accept().await.unwrap();
            let mut s = stream.into_unix().unwrap();
            let mut reader = tokio::io::BufReader::new(&mut s);
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            s.write_all(b"{\"ok\":1}\n").await.unwrap();
        });
        // client
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let mut client = tokio::net::UnixStream::connect(&path2).await.unwrap();
        client.write_all(b"hello\n").await.unwrap();
        let mut reader = tokio::io::BufReader::new(&mut client);
        let mut resp = String::new();
        reader.read_line(&mut resp).await.unwrap();
        assert_eq!(resp.trim(), r#"{"ok":1}"#);
        server.await.unwrap();
    }
}
