mod types; // Import the types module

use types::{ LogBuffer}; // Import your custom types
use futures::StreamExt;
const BUFFER_SIZE: usize = 6; 

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	const WEBSOCKET_INFURA_ENDPOINT: &str = "wss://mainnet.infura.io/ws/v3/[YOURWEBSOCKET]";
	
	let web3 =
		web3::Web3::new(web3::transports::ws::WebSocket::new(WEBSOCKET_INFURA_ENDPOINT).await?);
	let contract_address = web3::types::H160::from_slice(
		&hex::decode("5777d92f208679db4b9778590fa3cab3ac9e2168").unwrap()[..],
	);
	let contract = web3::contract::Contract::from_json(
		web3.eth(),
		contract_address,
		include_bytes!("contracts/uniswap_pool_abi.json"),
	)?;
	let swap_event = contract.abi().events_by_name("Swap")?.first().unwrap();

	let mut block_stream = web3.eth_subscribe().subscribe_new_heads().await?;
    let mut log_buffer = LogBuffer::new(); 

	
	while let Some(Ok(block)) = block_stream.next().await {

		log_buffer.add_block(&block);
		if log_buffer.buffer.len() >= BUFFER_SIZE {
			log_buffer.process(&web3,&swap_event,&contract_address).await?; 
		}    
    }
	Ok(())
}

