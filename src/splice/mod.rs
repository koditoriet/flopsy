use std::os::fd::AsRawFd;
use tokio::net::TcpStream;

pub mod pipe;

pub(crate) async fn copy_data(pipe: &pipe::Pipe, src: &mut TcpStream, dst: &mut TcpStream) -> std::io::Result<()> {
    let src_fd = src.as_raw_fd();
    let dst_fd = dst.as_raw_fd();
    let max_tx_size = 16384;
    let mut count = 0;
    let mut status = Ok(());
    while count < max_tx_size {
        let result = pipe.splice_from(src_fd, max_tx_size);
        let bytes_read = match result {
            Ok(0) => { status = Err(connection_closed()); break },
            Ok(bytes_read) => bytes_read,
            Err(pipe::Error::EAgain) => { clear_read_ready_flag(src) ; break },
            Err(pipe::Error::Other(e)) => return Err(other_error(e)),
        };
        count += bytes_read;
    }

    while count > 0 {
        let bytes_written = match pipe.splice_into(dst_fd, count) {
            Ok(0) => return Err(connection_closed()),
            Ok(bytes_written) => bytes_written,
            Err(pipe::Error::EAgain) => 0,
            Err(pipe::Error::Other(e)) => return Err(other_error(e))
        };
        count -= bytes_written;
    }
    status
}

fn clear_read_ready_flag(stream: &mut TcpStream) {
    stream.try_read(&mut [0;0]).unwrap_or(0);
}

fn connection_closed() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection closed")
}

fn other_error(errno: i32) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, format!("{}", errno))
}