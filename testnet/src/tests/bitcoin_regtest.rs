use std::process::{Command, Child};

use crate::{Config};
use crate::helium::RunLoop;

use stacks::chainstate::burn::operations::BlockstackOperationType::{LeaderBlockCommit, LeaderKeyRegister};
use stacks::util::hash::{hex_bytes};
use stacks::util::sleep_ms;

use super::{PUBLISH_CONTRACT};

pub enum BitcoinCoreError {
    SpawnFailed(String)
}

type BitcoinResult<T> = Result<T, BitcoinCoreError>;

pub struct BitcoinCoreController {
    bitcoind_process: Option<Child>,
    config: Config,
}

impl BitcoinCoreController {

    pub fn new(config: Config) -> BitcoinCoreController {
        BitcoinCoreController {
            bitcoind_process: None,
            config
        }
    }

    pub fn start_bitcoind(&mut self) -> BitcoinResult<()> {
        std::fs::create_dir_all(&self.config.get_burnchain_path()).unwrap();
        
        let mut command = Command::new("bitcoind");
        command
            // .stdout(Stdio::piped())
            .arg("-conf=/dev/null") // todo(ludo): nix only
            .arg("-regtest")
            .arg("-nodebug")
            .arg("-nodebuglogfile")
            .arg("-rest")
            .arg("-txindex=1")
            .arg("-server=1")
            .arg("-listenonion=0")
            .arg(&format!("-port={}", self.config.burnchain.peer_port))
            .arg(&format!("-datadir={}", self.config.get_burnchain_path()))
            .arg(&format!("-rpcport={}", self.config.burnchain.rpc_port));

        match (&self.config.burnchain.username, &self.config.burnchain.password) {
            (Some(username), Some(password)) => {
                command
                    .arg(&format!("-rpcuser={}", username))
                    .arg(&format!("-rpcpassword={}", password));
            },
            _ => {}
        }

        let process = match command.spawn() {
            Ok(child) => child,
            Err(e) => return Err(BitcoinCoreError::SpawnFailed(format!("{:?}", e)))
        };

        self.bitcoind_process = Some(process);

        Ok(())
    }

    pub fn kill_bitcoind(&mut self) {
        if let Some(mut bitcoind_process) = self.bitcoind_process.take() {
            bitcoind_process.kill().unwrap();
        }
    }
}

impl Drop for BitcoinCoreController {

    fn drop(&mut self) {
        self.kill_bitcoind();
    }
}

#[test]
#[ignore]
fn bitcoind_integration_test() {

    let mut conf = super::new_test_conf();
    conf.burnchain.commit_anchor_block_within = 2000;
    conf.burnchain.burn_fee_cap = 5000;
    conf.burnchain.mode = "helium".to_string();
    conf.burnchain.peer_host = "127.0.0.1".to_string();
    conf.burnchain.rpc_port = 18443;
    conf.burnchain.username = Some("helium-node".to_string());
    conf.burnchain.password = Some("secret".to_string());
    conf.burnchain.local_mining_public_key = Some("04ee0b1602eb18fef7986887a7e8769a30c9df981d33c8380d255edef003abdcd243a0eb74afdf6740e6c423e62aec631519a24cf5b1d62bf8a3e06ddc695dcb77".to_string());

    // Setup up a bitcoind controller
    let mut controller = BitcoinCoreController::new(conf.clone());
    // Start bitcoind
    let _res = controller.start_bitcoind();

    let num_rounds = 6;
    let mut run_loop = RunLoop::new(conf);

    run_loop.callbacks.on_burn_chain_initialized(|burnchain_controller| {
        // todo(ludo): we need to wait for bitcoind to be ready.
        sleep_ms(5000);
        burnchain_controller.bootstrap_chain();
    });

    // In this serie of tests, the callback is fired post-burnchain-sync, pre-stacks-sync
    run_loop.callbacks.on_new_burn_chain_state(|round, burnchain_tip, chain_tip| {
        match round {
            0 => {
                let block = &burnchain_tip.block_snapshot;
                assert!(block.block_height == 203);
                assert!(block.total_burn == 5000);
                assert!(block.num_sortitions == 1);
                assert!(block.sortition == true);

                let state_transition = &burnchain_tip.state_transition;
                assert!(state_transition.accepted_ops.len() == 2);
                assert!(state_transition.consumed_leader_keys.len() == 1);

                for op in &state_transition.accepted_ops {
                    match op {
                        LeaderKeyRegister(op) => {
                            assert!(op.public_key.to_hex() == "99fe9d43bbb0d36a23e4102cef59accfa983a342ae1e5acedc1b8dcb06b17cd4");
                        },
                        LeaderBlockCommit(op) => {
                            assert!(op.parent_block_ptr == 0);
                            assert!(op.parent_vtxindex == 0);
                            assert!(op.burn_fee == 5000);
                        }
                        _ => assert!(false)
                    }
                }
            },
            1 => {
                let block = &burnchain_tip.block_snapshot;
                assert!(block.block_height == 204);
                assert!(block.total_burn == 10000);
                assert!(block.num_sortitions == 2);
                assert!(block.sortition == true);

                let state_transition = &burnchain_tip.state_transition;
                assert!(state_transition.accepted_ops.len() == 2);
                assert!(state_transition.consumed_leader_keys.len() == 1);

                for op in &state_transition.accepted_ops {
                    match op {
                        LeaderKeyRegister(op) => {
                            assert!(op.public_key.to_hex() == "f6fb508bdbeb8c64faf4a376d773c4b6514874d8e508a30aa9ee4db86c6b7e8e");
                        },
                        LeaderBlockCommit(op) => {
                            assert!(op.parent_block_ptr == 203);
                            assert!(op.burn_fee == 5000);
                        }
                        _ => assert!(false)
                    }
                }

                assert!(burnchain_tip.block_snapshot.parent_burn_header_hash == chain_tip.metadata.burn_header_hash);
            },
            2 => {
                let block = &burnchain_tip.block_snapshot;
                assert!(block.block_height == 205);
                assert!(block.total_burn == 15000);
                assert!(block.num_sortitions == 3);
                assert!(block.sortition == true);

                let state_transition = &burnchain_tip.state_transition;
                assert!(state_transition.accepted_ops.len() == 2);
                assert!(state_transition.consumed_leader_keys.len() == 1);

                for op in &state_transition.accepted_ops {
                    match op {
                        LeaderKeyRegister(op) => {
                            assert!(op.public_key.to_hex() == "e89fc82ea3b5cfeab082e3a3294a6fbc6b9bb9a18d8179898898c2acafe21ab0");
                        },
                        LeaderBlockCommit(op) => {
                            assert!(op.parent_block_ptr == 204);
                            assert!(op.burn_fee == 5000);
                        }
                        _ => assert!(false)
                    }
                }           
            
                assert!(burnchain_tip.block_snapshot.parent_burn_header_hash == chain_tip.metadata.burn_header_hash);
            },
            3 => {
                let block = &burnchain_tip.block_snapshot;
                assert!(block.block_height == 206);
                assert!(block.total_burn == 20000);
                assert!(block.num_sortitions == 4);
                assert!(block.sortition == true);

                let state_transition = &burnchain_tip.state_transition;
                assert!(state_transition.accepted_ops.len() == 2);
                assert!(state_transition.consumed_leader_keys.len() == 1);

                for op in &state_transition.accepted_ops {
                    match op {
                        LeaderKeyRegister(op) => {
                            assert!(op.public_key.to_hex() == "c44604de9b87ee911db176c8d416c53c6aa046aa4be88333f5bc74d7f7c5a561");
                        },
                        LeaderBlockCommit(op) => {
                            assert!(op.parent_block_ptr == 205);
                            assert!(op.burn_fee == 5000);
                        }
                        _ => assert!(false)
                    }
                }            

                assert!(burnchain_tip.block_snapshot.parent_burn_header_hash == chain_tip.metadata.burn_header_hash);
            },
            4 => {
                let block = &burnchain_tip.block_snapshot;
                assert!(block.block_height == 207);
                assert!(block.total_burn == 25000);
                assert!(block.num_sortitions == 5);
                assert!(block.sortition == true);

                let state_transition = &burnchain_tip.state_transition;
                assert!(state_transition.accepted_ops.len() == 2);
                assert!(state_transition.consumed_leader_keys.len() == 1);

                for op in &state_transition.accepted_ops {
                    match op {
                        LeaderKeyRegister(op) => {
                            assert!(op.public_key.to_hex() == "cb58759fcb51972f98b53d755f5980695e5a0abd0e307002f32e8d73b40c2019");
                        },
                        LeaderBlockCommit(op) => {
                            assert!(op.parent_block_ptr == 206);
                            assert!(op.burn_fee == 5000);
                        }
                        _ => assert!(false)
                    }
                }

                assert!(burnchain_tip.block_snapshot.parent_burn_header_hash == chain_tip.metadata.burn_header_hash);
            },
            5 => {
                let block = &burnchain_tip.block_snapshot;
                assert!(block.block_height == 208);
                assert!(block.total_burn == 30000);
                assert!(block.num_sortitions == 6);
                assert!(block.sortition == true);

                let state_transition = &burnchain_tip.state_transition;
                assert!(state_transition.accepted_ops.len() == 2);
                assert!(state_transition.consumed_leader_keys.len() == 1);

                for op in &state_transition.accepted_ops {
                    match op {
                        LeaderKeyRegister(op) => {
                            assert!(op.public_key.to_hex() == "848a6ec4da123bd44b79e05dd8eb90fcfb92b985812cb54c886987f7dd54ac90");
                        },
                        LeaderBlockCommit(op) => {
                            assert!(op.parent_block_ptr == 207);
                            assert!(op.burn_fee == 5000);
                        }
                        _ => assert!(false)
                    }
                }
                
                assert!(burnchain_tip.block_snapshot.parent_burn_header_hash == chain_tip.metadata.burn_header_hash);
            },
            _ => {}
        }
    });

    // Use tenure's hook for submitting transactions
    run_loop.callbacks.on_new_tenure(|round, _burnchain_tip, chain_tip, tenure| {
        match round {
            1 => {
                // On round 1, publish the KV contract
                // $ cat /tmp/out.clar 
                // (define-map store ((key (buff 32))) ((value (buff 32))))
                // (define-public (get-value (key (buff 32)))
                //     (begin
                //         (print (concat "Getting key " key))
                //         (match (map-get? store ((key key)))
                //             entry (ok (get value entry))
                //             (err 0))))
                // (define-public (set-value (key (buff 32)) (value (buff 32)))
                //     (begin
                //         (print (concat "Setting key " key))
                //         (map-set store ((key key)) ((value value)))
                //         (ok 'true)))
                // ./blockstack-cli --testnet publish 043ff5004e3d695060fa48ac94c96049b8c14ef441c50a184a6a3875d2a000f3 0 0 store /tmp/out.clar
                let header_hash = chain_tip.block.block_hash();
                let burn_header_hash = chain_tip.metadata.burn_header_hash;
                tenure.mem_pool.submit_raw(&burn_header_hash, &header_hash, PUBLISH_CONTRACT.to_owned()).unwrap();
            },
            2 => {
                // On round 2, publish a "get:foo" transaction
                // ./blockstack-cli --testnet contract-call 043ff5004e3d695060fa48ac94c96049b8c14ef441c50a184a6a3875d2a000f3 0 1 STGT7GSMZG7EA0TS6MVSKT5JC1DCDFGZWJJZXN8A store get-value -e \"foo\"
                let header_hash = chain_tip.block.block_hash();
                let burn_header_hash = chain_tip.metadata.burn_header_hash;
                let get_foo = "8000000000040021a3c334fc0ee50359353799e8b2605ac6be1fe4000000000000000100000000000000000100e11fa0938e579c868137cfdd95fc0d6107a32c7a8864bbff2852c792c1759a38314e42922702b709c7b17c93d406f9d8057fb7c14736e5d85ff24acf89e921d6030200000000021a21a3c334fc0ee50359353799e8b2605ac6be1fe40573746f7265096765742d76616c7565000000010200000003666f6f";
                tenure.mem_pool.submit_raw(&burn_header_hash, &header_hash,hex_bytes(get_foo).unwrap().to_vec()).unwrap();
            },
            3 => {
                // On round 3, publish a "set:foo=bar" transaction
                // ./blockstack-cli --testnet contract-call 043ff5004e3d695060fa48ac94c96049b8c14ef441c50a184a6a3875d2a000f3 0 1 STGT7GSMZG7EA0TS6MVSKT5JC1DCDFGZWJJZXN8A store set-value -e \"foo\" -e \"bar\"
                let header_hash = chain_tip.block.block_hash();
                let burn_header_hash = chain_tip.metadata.burn_header_hash;
                let set_foo_bar = "8000000000040021a3c334fc0ee50359353799e8b2605ac6be1fe40000000000000002000000000000000001012409d25688e8101db21c1193b068a688d8c78fd120e87521e3e39887bbe7678b52f861ea5b798cc91642ee7e73a2135186d3f211194628d22ad8f433a3e56e31030200000000021a21a3c334fc0ee50359353799e8b2605ac6be1fe40573746f7265097365742d76616c7565000000020200000003666f6f0200000003626172";
                tenure.mem_pool.submit_raw(&burn_header_hash, &header_hash,hex_bytes(set_foo_bar).unwrap().to_vec()).unwrap();
            },
            4 => {
                // On round 4, publish a "get:foo" transaction
                // ./blockstack-cli --testnet contract-call 043ff5004e3d695060fa48ac94c96049b8c14ef441c50a184a6a3875d2a000f3 0 3 STGT7GSMZG7EA0TS6MVSKT5JC1DCDFGZWJJZXN8A store get-value -e \"foo\"
                let header_hash = chain_tip.block.block_hash();
                let burn_header_hash = chain_tip.metadata.burn_header_hash;
                let get_foo = "8000000000040021a3c334fc0ee50359353799e8b2605ac6be1fe40000000000000003000000000000000001014b327858d4a83c6cb4fb44021910c1ece6c1caf9cdefa13368ee004bca4558ff6c362ab66b0c416dbb7d54cb7e879debe1b27962e33569a5d8465345ab0a92c3030200000000021a21a3c334fc0ee50359353799e8b2605ac6be1fe40573746f7265096765742d76616c7565000000010200000003666f6f";
                tenure.mem_pool.submit_raw(&burn_header_hash, &header_hash,hex_bytes(get_foo).unwrap().to_vec()).unwrap();
            },
            5 => {
                // On round 5, publish a stacks transaction
                // ./blockstack-cli --testnet token-transfer b1cf9cee5083f421c84d7cb53be5edf2801c3c78d63d53917aee0bdc8bd160ee01 0 0 ST195Q2HPXY576N4CT2A0R94D7DRYSX54A5X3YZTH 1000
                let header_hash = chain_tip.block.block_hash();
                let burn_header_hash = chain_tip.metadata.burn_header_hash;
                let transfer_1000_stx = "80000000000400b71a091b4b8b7661a661c620966ab6573bc2dcd30000000000000000000000000000000000001ebe25170de7d487c0eb7185281ebeb57e7e329111d72b520e02b9e7e817e46b7e0c9468397aec94f9540c6a2791063bad37dc1322f120642f307a79fa76b5f103020000000000051a525b8a36ef8a73548cd0940c248d3b71ecf4a45100000000000003e800000000000000000000000000000000000000000000000000000000000000000000";
                tenure.mem_pool.submit_raw(&burn_header_hash, &header_hash,hex_bytes(transfer_1000_stx).unwrap().to_vec()).unwrap();
            },
            _ => {}
        };
        return
    });

    // Use block's hook for asserting expectations
    // In this serie of tests, the callback is fired post-burnchain-sync, post-stacks-sync
    run_loop.callbacks.on_new_stacks_chain_state(|round, burnchain_tip, chain_tip, _chain_state| {
        match round {
            0 => {
                // Inspecting the chain at round 0.
                // - Chain length should be 1.
                assert!(chain_tip.metadata.block_height == 1);
                
                // Block #1 should only have 0 txs
                assert!(chain_tip.block.txs.len() == 1);

                assert!(chain_tip.block.header.block_hash() == burnchain_tip.block_snapshot.winning_stacks_block_hash);
            },
            1 => {
                // Inspecting the chain at round 1.
                // - Chain length should be 2.
                assert!(chain_tip.metadata.block_height == 2);
                
                // Block #2 should only have 2 txs
                assert!(chain_tip.block.txs.len() == 2);

                assert!(chain_tip.block.header.block_hash() == burnchain_tip.block_snapshot.winning_stacks_block_hash);

            },
            2 => {
                // Inspecting the chain at round 2.
                // - Chain length should be 3.
                assert!(chain_tip.metadata.block_height == 3);
                
                // Block #3 should only have 2 txs
                assert!(chain_tip.block.txs.len() == 2);

                assert!(chain_tip.block.header.block_hash() == burnchain_tip.block_snapshot.winning_stacks_block_hash);

            },
            3 => {
                // Inspecting the chain at round 3.
                // - Chain length should be 4.
                assert!(chain_tip.metadata.block_height == 4);
                
                // Block #4 should only have 2 txs
                assert!(chain_tip.block.txs.len() == 2);

                assert!(chain_tip.block.header.block_hash() == burnchain_tip.block_snapshot.winning_stacks_block_hash);
            },
            4 => {
                // Inspecting the chain at round 4.
                // - Chain length should be 5.
                assert!(chain_tip.metadata.block_height == 5);
                
                // Block #5 should only have 2 txs
                assert!(chain_tip.block.txs.len() == 2);

                assert!(chain_tip.block.header.block_hash() == burnchain_tip.block_snapshot.winning_stacks_block_hash);

            },
            5 => {
                // Inspecting the chain at round 5.
                // - Chain length should be 6.
                assert!(chain_tip.metadata.block_height == 6);
                
                // Block #6 should only have 2 txs
                assert!(chain_tip.block.txs.len() == 2);

                assert!(chain_tip.block.header.block_hash() == burnchain_tip.block_snapshot.winning_stacks_block_hash);
            },
            _ => {}
        }
    });
    run_loop.start(num_rounds);

    controller.kill_bitcoind();
}
