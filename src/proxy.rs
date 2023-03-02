use std::path::PathBuf;
use tokio::{net::{TcpListener, TcpStream}, process::Command, io::AsyncWriteExt};
use crate::{args::Args, stream_util::bridge_streams};

pub struct Proxy {
    args: Args,
    primary_host: String
}

impl Proxy {
    /// Creates a new failover proxy.
    pub fn create(args: Args) -> Self {
        Self {
            primary_host: String::from(""),
            args: args,
        }
    }

    /// Starts the receiver failover proxy and keeps running perpetually.
    pub async fn run(mut self) {
        if self.args.lazy_init {
            eprintln!("lazy init requested; not selecting an initial primary");
            return
        } else {
            if !self.select_initial_primary().await {
                eprintln!("no primary available; exiting");
                return
            }
        }
        let listener = self.bind().await;
        loop {
            match listener.accept().await {
                Ok((conn, _)) => self.handle_connection(conn).await.unwrap_or(()),
                Err(err) => eprintln!("couldn't accept incoming connection: {}", err),
            }
        }
    }

    async fn select_initial_primary(&mut self) -> bool {
        match self.handle_failover().await {
            Ok(mut stream) => {
                stream.shutdown().await.unwrap();
                true
            }
            Err(_) => {
                false
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
            Err(error) => {
                eprintln!("primary '{}' unavailable: {}", self.primary_host, error);
                self.handle_failover().await
            },
        }
    }

    /// Selects a new primary and runs all failover triggers on it.
    async fn handle_failover(&mut self) -> std::io::Result<TcpStream> {
        eprintln!("selecting a new primary...");
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

    /// Returns true if the given host is eligible to be a primary,
    /// otherwise returns false.
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

    /// Run all failover triggers in turn, with the given connection string
    /// as their only argument.
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

/// Run the given executable with the given connection string as its only argument.
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

/// Returns a list of files found at the given path.
/// Either the path itself, if the path is a file, or all files contained in
/// the directory indicated by the path, if the path is a directory.
fn collect_files(path: &PathBuf) -> Vec<PathBuf> {
    if path.is_file() {
        vec![path.clone()]
    } else if path.is_dir() {
        let mut files = path.read_dir().unwrap()
            .filter_map(|x| {
                let path = x.unwrap().path();
                if path.is_file() {
                    Some(path)
                } else {
                    None
                }
            })
            .collect::<Vec<PathBuf>>();
        files.sort();
        files
    } else {
        eprintln!("path '{}' is neither a file nor a directory", path.to_str().unwrap());
        vec![]
    }
}
