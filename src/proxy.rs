use std::path::PathBuf;
use tokio::{net::{TcpListener, TcpStream}, process::Command, io::AsyncWriteExt};
use crate::{args::Args, stream_util::bridge_streams};

pub struct Proxy {
    args: Args,
    primary_host: String
}

impl Proxy {
    pub fn create(args: Args) -> Self {
        Self {
            primary_host: String::from(""),
            args: args,
        }
    }

    pub async fn run(mut self) {
        let listener = self.bind().await;
        loop {
            match listener.accept().await {
                Ok((conn, _)) => self.handle_connection(conn).await.unwrap_or(()),
                Err(err) => eprintln!("couldn't accept incoming connection: {}", err),
            }
        }
    }

    async fn bind(&self) -> TcpListener {
        match TcpListener::bind(self.args.bind_address()).await {
            Ok(l) => l,
            Err(err) => panic!("{}", err.to_string()),
        }
    }

    async fn connect_to_primary(&mut self) -> std::io::Result<TcpStream> {
        match TcpStream::connect(&self.primary_host).await {
            Ok(stream) => Ok(stream),
            Err(_) => self.handle_failover().await,
        }
    }

    async fn handle_failover(&mut self) -> std::io::Result<TcpStream> {
        eprintln!("primary host '{}' is unreachable; trying to find a new one", self.primary_host);
        for host in &self.args.hosts {
            eprintln!("trying host '{}'", host);
            let stream = match TcpStream::connect(host).await {
                Ok(stream) => stream,
                Err(_) => {
                    eprintln!("host '{}' discarded because it is unreachable", host);
                    continue
                },
            };
            if self.check_primary(host).await {
                eprintln!("host '{}' selected as the new primary", host);
                self.primary_host = host.clone();
                self.run_failover_triggers(host).await;
                return Ok(stream)
            } else {
                eprintln!("host '{}' discarded because it failed the primary check", host)
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::Other, "no primary found"))
    }

    async fn check_primary(&self, host: &String) -> bool {
        match &self.args.check_host {
            None => true,
            Some(command) => {
                let status = Command::new(command).arg(host).status().await;
                match status {
                    Ok(exit_status) if exit_status.success() => true,
                    _ => false
                }
            },
        }
    }

    async fn run_failover_triggers(&self, host: &String) {
        if let Some(path) = &self.args.on_failover {
            eprintln!("running failover triggers on host '{}'", self.primary_host);
            for trigger in collect_files(path) {
                run_trigger(&trigger, host).await
            }
        }
    }

    async fn handle_connection(&mut self, mut client_stream: TcpStream) -> std::io::Result<()> {
        match self.connect_to_primary().await {
            Ok(stream) => {
                tokio::spawn(bridge_streams(client_stream, stream));
                Ok(())
            },
            Err(error) => {
                eprintln!("no hosts available; shutting down client connection");
                client_stream.shutdown().await.unwrap_or(());
                Err(error)
            },
        }
    }
}

async fn run_trigger(trigger: &PathBuf, host: &String) {
    eprintln!("running trigger '{}' on host '{}'", trigger.to_str().unwrap(), host);
    match Command::new(&trigger).arg(host).status().await {
        Ok(exit_status) if !exit_status.success() => {
            eprintln!("trigger '{}' failed with status {}", trigger.to_str().unwrap(), exit_status)
        }
        Err(error) => {
            eprintln!("unable to call trigger '{}': {}", trigger.to_str().unwrap(), error)
        }
        _ => { /* trigger succeeded, yay */ }
    }
}

fn collect_files(path: &PathBuf) -> Vec<PathBuf> {
    if path.is_file() {
        vec![path.clone()]
    } else if path.is_dir() {
        let mut files = path.read_dir().unwrap()
            .map(|x| x.unwrap().path())
            .collect::<Vec<PathBuf>>();
        files.sort();
        files
    } else {
        eprintln!("path '{}' is neither a file nor a directory", path.to_str().unwrap());
        vec![]
    }
}
