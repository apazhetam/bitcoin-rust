use serde::Serialize;
use crate::blockchain::Blockchain;
use crate::miner::Handle as MinerHandle;
use crate::generator::Handle as GeneratorHandle;
use crate::network::server::Handle as NetworkServerHandle;
use crate::network::message::Message;
use crate::types::{
    mempool::Mempool,
    hash::{H256, Hashable},
    block::Content,
    transaction::SignedTransaction,
};

use log::info;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server as HTTPServer;
use url::Url;

pub struct Server {
    handle: HTTPServer,
    miner: MinerHandle,
    txn_generator: GeneratorHandle,
    network: NetworkServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let payload = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let resp = Response::from_string(serde_json::to_string_pretty(&payload).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}
macro_rules! respond_json {
    ( $req:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let resp = Response::from_string(serde_json::to_string(&$message).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}

impl Server {
    pub fn start(
        addr: std::net::SocketAddr,
        miner: &MinerHandle,
        txn_generator: &GeneratorHandle,
        network: &NetworkServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        mempool: &Arc<Mutex<Mempool>>
    ) {
        let handle = HTTPServer::http(&addr).unwrap();
        let server = Self {
            handle,
            miner: miner.clone(),
            txn_generator: txn_generator.clone(),
            network: network.clone(),
            blockchain: Arc::clone(blockchain),
            mempool: Arc::clone(mempool),
        };
        thread::spawn(move || {
            for req in server.handle.incoming_requests() {
                let miner = server.miner.clone();
                let txn_generator = server.txn_generator.clone();
                let network = server.network.clone();
                let blockchain = Arc::clone(&server.blockchain);
                let mempool = Arc::clone(&server.mempool);
                thread::spawn(move || {
                    // a valid url requires a base
                    let base_url = Url::parse(&format!("http://{}/", &addr)).unwrap();
                    let url = match base_url.join(req.url()) {
                        Ok(u) => u,
                        Err(e) => {
                            respond_result!(req, false, format!("error parsing url: {}", e));
                            return;
                        }
                    };
                    match url.path() {
                        "/miner/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let lambda = match params.get("lambda") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing lambda");
                                    return;
                                }
                            };
                            let lambda = match lambda.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing lambda: {}", e)
                                    );
                                    return;
                                }
                            };
                            miner.start(lambda);
                            respond_result!(req, true, "ok");
                        }
                        "/tx-generator/start" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let theta = match params.get("theta") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing theta");
                                    return;
                                }
                            };
                            let theta = match theta.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing theta: {}", e)
                                    );
                                    return;
                                }
                            };
                            txn_generator.start(theta);
                            respond_result!(req, true, "ok");
                        }
                        "/network/ping" => {
                            network.broadcast(Message::Ping(String::from("Test ping")));
                            respond_result!(req, true, "ok");
                        }
                        "/blockchain/longest-chain" => {
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            let v_string: Vec<String> = v.into_iter().map(|h|h.to_string()).collect();
                            respond_json!(req, v_string);
                        }
                        "/blockchain/longest-chain-tx" => {
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            
                            let mut txn_chain = Vec::new();     // will store all transactions
                            for block_hash in v {               // iterate through all blocks
                                let block = blockchain.get_block(&block_hash).unwrap().clone();
                                let mut block_txns = Vec::new();
                                for txn in block.content.transactions.iter() {
                                    block_txns.push(txn.hash().to_string());
                                }
                                txn_chain.push(block_txns);     // add block's txns to txn_chain
                            }
                            drop(blockchain);
                    
                            respond_json!(req, txn_chain);
                        }
                        "/blockchain/longest-chain-tx-count" => {
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            
                            let mut total_tx_count = 0; // will store total # of transactions
                            for block_hash in v {       // iterate through all blocks
                                let block = blockchain.get_block(&block_hash).unwrap().clone();
                                total_tx_count += block.content.transactions.len(); // add block's transactions count
                            }
                            drop(blockchain);
                            
                            respond_json!(req, total_tx_count);
                        }
                        "/blockchain/state" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let block_num = match params.get("block") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing block number");
                                    return;
                                }
                            };
                            let block_num = match block_num.parse::<usize>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing block number: {}", e)
                                    );
                                    return;
                                }
                            };

                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            
                            // Handle block_num values that are out of bounds
                            if block_num >= v.len() {
                                respond_result!(req, false, "given block number is out of bounds");
                                return;
                            }

                            // Get state
                            let block_hash = v[block_num];
                            let state = match blockchain.get_state(&block_hash) {
                                Ok(s) => s.clone(),
                                Err(_) => panic!("Block missing from blockchain.")
                            };
                            drop(blockchain);
                            
                            let mut acc_info = Vec::new();
                            for (address, (acc_nonce, balance)) in &state.map {
                                let address_str = address.clone().to_hex_string();
                                let info_str = format!("({}, {}, {})", address_str, acc_nonce, balance);
                                acc_info.push(info_str);
                            }
                            acc_info.sort();

                            respond_json!(req, acc_info);
                        }
                        "/blockchain/num-blocks" => {
                            let blockchain = blockchain.lock().unwrap();
                            let length = blockchain.all_blocks_in_longest_chain().len();
                            respond_json!(req, length);
                        }
                        "/mempool" => {
                            let mempool = mempool.lock().unwrap();
                            let map = mempool.map.clone();
                            drop(mempool);

                            let mut all_txns = Vec::new();
                            for txn in map.values() {
                                let acc_nonce = txn.transaction.account_nonce;
                                let receiver = txn.transaction.receiver.clone().to_hex_string();
                                let value = txn.transaction.value;
                                let info = (acc_nonce, receiver, value);
                                all_txns.push(info);
                            }
                            
                            respond_json!(req, all_txns);
                        }
                        _ => {
                            let content_type =
                                "Content-Type: application/json".parse::<Header>().unwrap();
                            let payload = ApiResponse {
                                success: false,
                                message: "endpoint not found".to_string(),
                            };
                            let resp = Response::from_string(
                                serde_json::to_string_pretty(&payload).unwrap(),
                            )
                            .with_header(content_type)
                            .with_status_code(404);
                            req.respond(resp).unwrap();
                        }
                    }
                });
            }
        });
        info!("API server listening at {}", &addr);
    }
}
