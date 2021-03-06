extern crate rand;
extern crate mio;
extern crate serde;

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
#[macro_use] extern crate stacks;

pub use stacks::util;

pub mod run_loop; 
pub mod keychain;
pub mod node;
pub mod tenure;
pub mod config;
pub mod event_dispatcher;
pub mod operations;
pub mod burnchains;
pub mod neon_node;

pub use self::keychain::{Keychain};
pub use self::node::{Node, ChainTip};
pub use self::neon_node::{InitializedNeonNode, NeonGenesisNode};
pub use self::burnchains::{MocknetController, BitcoinRegtestController, BurnchainTip, BurnchainController};
pub use self::tenure::{Tenure};
pub use self::config::{Config, ConfigFile};
pub use self::event_dispatcher::{EventDispatcher};
pub use self::run_loop::{neon, helium};

use pico_args::Arguments;
use std::env;
    
fn main() {

    let mut args = Arguments::from_env();
    let subcommand = args.subcommand().unwrap().unwrap_or_default();

    let config_file = match subcommand.as_str() {
        "mocknet" => {
            args.finish().unwrap();
            ConfigFile::mocknet()
        }
        "helium" => {
            args.finish().unwrap();
            ConfigFile::helium()
        }
        "neon" => {
            args.finish().unwrap();
            ConfigFile::neon()
        }
        "start" => {
            let config_path: String = args.value_from_str("--config").unwrap();
            args.finish().unwrap();
            println!("==> {}", config_path);
            ConfigFile::from_path(&config_path)
        }
        "version" => {
            print_version();
            return;
        }
        _ => {
            print_help();
            return
        }
    };

    let conf = Config::from_config_file(config_file);

    let num_round: u64 = 0; // Infinite number of rounds

    if conf.burnchain.mode == "helium" || conf.burnchain.mode == "mocknet" {
        let mut run_loop = helium::RunLoop::new(conf);
        run_loop.start(num_round);
    } else if conf.burnchain.mode == "neon" {
        let mut run_loop = neon::RunLoop::new(conf);
        run_loop.start(num_round);
    } else {
        println!("Burnchain mode '{}' not supported", conf.burnchain.mode);
    }
}

fn print_help() {
    let argv: Vec<_> = env::args().collect();

    eprintln!(
        "\
{} <SUBCOMMAND>
Run a stacks-node.

USAGE:
stacks-node <SUBCOMMAND>

SUBCOMMANDS:

mocknet\t\tStart a node based on a fast local setup emulating a burnchain. Ideal for smart contract development. 

helium\t\tStart a node based on a local setup relying on a local instance of bitcoind.
\t\tThe following bitcoin.conf is expected:
\t\t  chain=regtest
\t\t  disablewallet=0
\t\t  txindex=1
\t\t  server=1
\t\t  rpcuser=helium
\t\t  rpcpassword=helium

neon\t\tStart a node that will join and stream blocks from the public neon testnet, powered by Blockstack.

start\t\tStart a node with a config of your own. Can be used for joining a network, starting new chain, etc.
\t\tArguments:
\t\t  --config: path of the config (such as https://github.com/blockstack/stacks-blockchain/blob/master/testnet/Stacks.toml).
\t\tExample:
\t\t  stacks-node start --config=/path/to/config.toml

version\t\tDisplay informations about the current version and our release cycle.

help\t\tDisplay this help.

", argv[0]);
}

fn print_version() {
    println!("2020-04-23");
}

#[cfg(test)]
pub mod tests;
