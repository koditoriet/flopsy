use tokio::{net::TcpStream, io::{AsyncWriteExt}, select};

use crate::splice::{pipe::Pipe, copy_data};

pub(crate) async fn bridge_streams(mut client_stream: TcpStream, mut server_stream: TcpStream) {
    client_stream.set_nodelay(true).unwrap_or(());
    server_stream.set_nodelay(true).unwrap_or(());
    let pipe = Pipe::new();
    loop {
        let (r, w) = select_readable(&mut client_stream, &mut server_stream).await;
        if let Err(e) = copy_data(&pipe, r, w).await {
            eprintln!("{}", e);
            break
        }
    }
    client_stream.shutdown().await.unwrap_or(());
    server_stream.shutdown().await.unwrap_or(());
}

#[inline(always)]
async fn select_readable<'a>(a: &'a mut TcpStream, b: &'a mut TcpStream) -> (&'a mut TcpStream, &'a mut TcpStream) {
    select! {
        _ = a.readable() => return (a, b),
        _ = b.readable() => return (b, a),
    }
}

async fn copy_data_buf(_pipe: &Pipe, src: &TcpStream, dst: &mut TcpStream) -> std::io::Result<()> {
    let mut buf = [0u8; 16384];
    match src.try_read(&mut buf) {
        Ok(0) => Err(std::io::Error::new(std::io::ErrorKind::Other, "no more data")),
        Ok(bytes_read) => { dst.write(&buf[0..bytes_read]).await? ; Ok(()) },
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e),
    }
}
