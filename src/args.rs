use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    /// Interface and port to bind.
    #[arg(short, long, required_unless_present("port"), conflicts_with="port")]
    bind: Option<String>,

    /// Port to bind on all interfaces. Equivalent to --bind [::]:<PORT>.
    #[arg(short, long, required_unless_present("bind"))]
    port: Option<u16>,

    /// Comma-delimited list of hosts to connect to.
    #[arg(short='H', long, required=true, value_delimiter=',')]
    pub hosts: Vec<String>,

    /// Shell script to execute to figure out whether a given host is a primary.
    /// 
    /// The script is passed the host's connection string as its only argument.
    /// If the script exits with a non-zero value, the host will not be considered a primary.
    /// The script is only called if the host is accepting connections on the service port.
    /// 
    /// If no script is given, the first host accepting connections on the service port is
    /// considered the primary.
    #[arg(short, long)]
    pub check_host: Option<PathBuf>,

    /// Shell script or directory of shell scripts to execute when a failover is performed.
    /// 
    /// When a new primary is found to be unavailable, this script is called with the new primary's
    /// connection string as its only argument. No new connections are accepted by flopsy
    /// until all scripts called this way have terminated.
    /// 
    /// If the given path is a directory instead of a shell script, all scripts in the directory will
    /// be called in alphabetical order before new connections are accepted.
    #[arg(short='f', long)]
    pub on_failover: Option<PathBuf>
}

impl Args {
    pub fn bind_address(&self) -> String {
        self.port
            .map(|x| format!("[::]:{}", x))
            .or_else(||self.bind.clone())
            .expect("either bind or port must be set")
    }
}