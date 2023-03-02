use std::{path::PathBuf, time::Duration, cmp::min};
use tokio::{net::{TcpListener, TcpStream}, process::Command, io::AsyncWriteExt, time::sleep};
use crate::{args::Args, stream_util::bridge_streams};

pub struct Proxy {
    args: Args,
    primary_host: String,
    listener: Option<TcpListener>,
}

impl Proxy {
    /// Creates a new failover proxy.
    pub fn create(args: Args) -> Self {
        Self {
            primary_host: "".to_string(),
            args: args,
            listener: None,
        }
    }

    /// Starts the receiver failover proxy and keeps running perpetually.
    pub async fn run(mut self) {
        self.handle_failover().await;
        loop {
            match self.listener.as_ref().unwrap().accept().await {
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

    /// Selects a new primary and runs all failover triggers on it.
    /// Blocks indefinitely until a new primary can be selected.
    async fn handle_failover(&mut self) {
        eprintln!("selecting a new primary...");
        self.listener = None;
        let mut sleep_duration = Duration::from_secs(1);
        let max_sleep_duration = Duration::from_secs(self.args.max_primary_selection_backoff_secs as u64);
        loop {
            for host in &self.args.hosts {
                eprintln!("trying host '{}'", host);
                match TcpStream::connect(host).await {
                    Ok(mut stream) => stream.shutdown().await.unwrap_or(()),
                    Err(_) => {
                        eprintln!("host '{}' discarded because it is unreachable", host);
                        continue
                    },
                };
                if self.check_primary(host).await {
                    eprintln!("host '{}' selected as the new primary", host);
                    self.run_failover_triggers(host).await;
                    self.primary_host = host.clone();
                    self.listener = Some(self.bind().await);
                    return
                } else {
                    eprintln!("host '{}' discarded because it failed the primary check", host)
                }
            }
            eprintln!("no hosts available; sleeping for {} seconds, then trying again", sleep_duration.as_secs());
            sleep(sleep_duration).await;
            sleep_duration = min(max_sleep_duration, sleep_duration*2);
        }
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
            eprintln!("running failover triggers on host '{}'", host);
            for trigger in collect_files(path) {
                run_trigger(&trigger, host).await
            }
        }
    }

    async fn handle_connection(&mut self, mut client_stream: TcpStream) -> std::io::Result<()> {
        match TcpStream::connect(&self.primary_host).await {
            Ok(stream) => {
                tokio::spawn(bridge_streams(client_stream, stream));
                Ok(())
            },
            Err(error) => {
                eprintln!("couldn't connect to primary; shutting down client connection");
                client_stream.shutdown().await.unwrap_or(());
                drop(client_stream);
                self.handle_failover().await;
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
