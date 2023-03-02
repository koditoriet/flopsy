use tokio::{net::TcpStream, io::{AsyncWriteExt}, select};

#[cfg(feature = "splice")]
use crate::splice::{copy_data, create_pipe};

#[cfg(not(feature = "splice"))]
use crate::bufcopy::{copy_data, create_pipe};

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

#[inline(always)]
async fn select_readable<'a>(a: &'a mut TcpStream, b: &'a mut TcpStream) -> (&'a mut TcpStream, &'a mut TcpStream) {
    select! {
        _ = a.readable() => return (a, b),
        _ = b.readable() => return (b, a),
    }
}
