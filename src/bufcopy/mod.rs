use tokio::{net::TcpStream, io::AsyncWriteExt};

pub(crate) type Pipe = ();

pub(crate) fn create_pipe() -> Pipe {
    ()
}

pub(crate) async fn copy_data(_pipe: &Pipe, src: &TcpStream, dst: &mut TcpStream) -> std::io::Result<()> {
    let mut buf = [0u8; 16384];
    match src.try_read(&mut buf) {
        Ok(0) => Err(std::io::Error::new(std::io::ErrorKind::Other, "connection closed")),
        Ok(bytes_read) => { dst.write(&buf[0..bytes_read]).await? ; Ok(()) },
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e),
    }
}