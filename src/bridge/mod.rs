//! The Yana bridge: one local hub that Yana's apps connect to.
//!
//! Design: .planning/yana-bridge/SPEC.md. So far: the wire format, the version
//! handshake and delivery between connected apps. Nothing is stored yet, and
//! the `control` stream is refused until apps can be told apart.

pub mod protocol;

#[cfg(unix)]
mod hub;
#[cfg(unix)]
mod outbox;
#[cfg(unix)]
mod server;

#[cfg(unix)]
pub use cli::{run, BridgeAction};

#[cfg(unix)]
mod cli {
    use super::{hub::Hub, server};
    use anyhow::{Context, Result};
    use clap::Subcommand;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[derive(Subcommand)]
    pub enum BridgeAction {
        /// Serve the local bridge until stopped (experimental)
        Serve {
            /// Directory of the socket. Default: ~/.yana/bridge
            #[arg(long)]
            dir: Option<PathBuf>,
        },
    }

    pub fn run(action: BridgeAction) -> Result<()> {
        let BridgeAction::Serve { dir } = action;
        let dir = match dir {
            Some(dir) => dir,
            None => {
                let home = std::env::var_os("HOME").context("HOME is not set; pass --dir")?;
                PathBuf::from(home).join(".yana").join("bridge")
            }
        };
        let (listener, path) = server::bind(&dir)
            .with_context(|| format!("cannot open the bridge socket in {}", dir.display()))?;
        eprintln!("yana-rt bridge: listening on {}", path.display());
        server::serve(listener, Arc::new(Hub::default()))?;
        Ok(())
    }
}
