use tokio::{net::TcpStream, io::{AsyncWriteExt}, select};

#[cfg(feature = "splice")]
use crate::splice::{copy_data, create_pipe};

#[cfg(not(feature = "splice"))]
use crate::bufcopy::{copy_data, create_pipe};

/// Forwards data from client_stream to server_stream and vice versa,
/// until either stream is closed on the other end.
pub(crate) async fn bridge_streams(mut client_stream: TcpStream, mut server_stream: TcpStream) {
    client_stream.set_nodelay(true).unwrap_or(());
    server_stream.set_nodelay(true).unwrap_or(());
    let pipe = create_pipe();

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

/// Waits for one of the given streams to become readable,
/// then returns (readable_stream, other_stream).
#[inline(always)]
async fn select_readable<'a>(a: &'a mut TcpStream, b: &'a mut TcpStream) -> (&'a mut TcpStream, &'a mut TcpStream) {
    select! {
        _ = a.readable() => return (a, b),
        _ = b.readable() => return (b, a),
    }
}
