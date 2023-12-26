use web3::{
    types::{BlockHeader, U64, Log, U256,H160},
	ethabi::{self, Event},
};
use std::collections::VecDeque;
use anyhow::Error;
const BUFFER_SIZE: usize = 6; 


pub struct SwapLog {
    sender: H160,
    recipient: H160,
    amount0: U256,
    amount1: U256,
	decimal0: f64,
	decimal1: f64,	
}
pub struct LogBuffer {
    pub buffer: VecDeque<BlockHeader>,
}


impl SwapLog {
    // Function to create a SwapLog from a raw Ethereum log
    fn from_log(swap_event_abi: &Event, log: &Log) -> Result<Self, Error> {
    
	
	 // Decode the log
	 let parsed_log = swap_event_abi.parse_log(ethabi::RawLog {
        topics: log.topics.clone(),
        data: log.data.0.clone(),
    })?;
  
    // Extract the details from the parsed log
     let sender = if let ethabi::Token::Address(addr) = &parsed_log.params[0].value {
       addr
   } else {
       return Err(anyhow::Error::msg("Expected sender address"));
   };

   let recipient = if let ethabi::Token::Address(addr) = &parsed_log.params[1].value {
       addr
   } else {
       return Err(anyhow::Error::msg("Expected recipient address"));
   };

   // Assuming amount0 and amount1 are of type Uint (U256)
   let amount0 = if let ethabi::Token::Int(value) = &parsed_log.params[2].value {
       value
   } else {
       return Err(anyhow::Error::msg("Expected amount0 uint"));
   };

   let amount1 = if let ethabi::Token::Int(value) = &parsed_log.params[3].value {
       value
   } else {
       return Err(anyhow::Error::msg("Expected amount1 uint"));
   };

        let decimal0 = Self::convert_if_negative(*amount0) as f64 / 1e18; // DAI precision
        let decimal1 = Self::convert_if_negative(*amount1) as f64 / 1e6;  // USDC precision

        Ok(SwapLog {
            sender: *sender,
            recipient: *recipient,
            amount0: *amount0,
            amount1: *amount1,
            decimal0,
            decimal1,
        })
    }
	    // Function to print swap log details
	fn print_details(&self) {
			
			let direction = if self.amount0 > U256::from(0) { 
                "DAI to USDC" 
            } else { 
                "USDC to DAI" 
            };
	
			println!("Swap Details:");
			println!("Sender: {:?}", self.sender);
			println!("Recipient: {:?}", self.recipient);
			println!("Direction: {}", direction);
			println!("Amounts: {} DAI, {} USDC", self.decimal0, self.decimal1);
	}
	fn convert_if_negative(value: U256) -> i128 {
		if value <= U256::from(i128::MAX) {
			// It's a positive number or small negative number that fits into i128
			value.low_u128() as i128
		} else {
			// It's a large negative number, convert from two's complement
            let inverted = !(value - 1);
            let lower_bits = inverted.low_u128();
            -(lower_bits as i128)
		}
	}
	
}

impl LogBuffer {
    pub fn new() -> Self {
        LogBuffer {
            buffer: VecDeque::with_capacity(BUFFER_SIZE + 1),
        }
    }

    pub fn add_block(&mut self, block: &BlockHeader) {
        self.buffer.push_back(block.clone());
        println!("Block :{:?} added to buffer, buffer size:{:?}", block.number.unwrap() ,self.buffer.len());
        if self.buffer.len() > 1 && self.detect_deep_reorganization().is_err() {
            #[cfg(not(test))]
            {
                println!("Deep reorganization detected Exiting.");
                std::process::exit(1);
            }
            #[cfg(test)]
            {
            println!("Exiting behavior suppressed during tests.");
            }
        }
    }

    pub async fn  process(&mut self,web3: &web3::Web3<web3::transports::ws::WebSocket>
        , swap_event: &Event,contract_address:&web3::types::H160
    ) -> Result<(), anyhow::Error> {
            let confirmed_block =  self.buffer.pop_front().unwrap();
            
            println!("Processing block number {}",confirmed_block.number.unwrap());
            let swap_logs_in_block = web3.eth().logs(
                web3::types::FilterBuilder::default()
                    .block_hash(confirmed_block.hash.unwrap())
                    .address(vec![*contract_address])
                    .topics(Some(vec![swap_event.signature()]), None, None, None)
                    .build(),
            ).await?;
                for log in swap_logs_in_block {
                    if let Err(err) = SwapLog::from_log(swap_event, &log) {
                        // Log the error and skip processing the block
                        println!("Error processing block: {}", err);
                        break;
                    } else if let Ok(swap_log) = SwapLog::from_log(swap_event,&log) {
                        swap_log.print_details();

                    }
            } 
        Ok(())
     
    }
    pub fn detect_deep_reorganization(&mut self) ->  Result<(), anyhow::Error> {
        if let Some(confirmed_block) = self.buffer.front() {
            if confirmed_block.number.unwrap() + U64::from(BUFFER_SIZE as u64) <= self.buffer.back().unwrap().number.unwrap() {
                return Err(anyhow::Error::msg("Deep reorganization detected"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use web3::types::{BlockHeader, H160, H256, U64, U256, Bytes, H2048, H64};
    // https://docs.rs/web3/latest/web3/types/struct.BlockHeader.html
    fn mock_block_header(block_number: u64) -> BlockHeader {
        BlockHeader {
            number: Some(U64::from(block_number)),  // Block number set from parameter
            hash: Some(H256::default()),            
            parent_hash: H256::default(),          
            uncles_hash: H256::default(),           
            author: H160::default(),                
            state_root: H256::default(),          
            transactions_root: H256::default(),    
            receipts_root: H256::default(),         
            gas_used: U256::default(),              
            gas_limit: U256::default(),             
            base_fee_per_gas: Some(U256::default()),
            extra_data: Bytes::default(),           
            logs_bloom: H2048::default(),           
            timestamp: U256::default(),             
            difficulty: U256::default(),            
            mix_hash: Some(H256::default()),        
            nonce: Some(H64::default()),           
        }
    }

    #[test]
    fn test_detect_deep_reorganization() {
        let mut log_buffer = LogBuffer::new();

        //No deep reorganization 
        for i in 1..=BUFFER_SIZE - 1 {
            log_buffer.add_block(&mock_block_header(i as u64));
        }
        assert!(log_buffer.detect_deep_reorganization().is_ok(), "Deep reorganization incorrectly detected in sequential blocks");

        //Deep reorganization detected during buffer initialization
        let mut log_buffer = LogBuffer::new(); // Reset buffer
        for i in 1..=BUFFER_SIZE / 2 {
            log_buffer.add_block(&mock_block_header(i as u64));
        }
        //triggers deep reorganization during initialization
        log_buffer.add_block(&mock_block_header(BUFFER_SIZE as u64 + 100));
        assert!(log_buffer.detect_deep_reorganization().is_err(), "Deep reorganization not detected during initialization");

        //Deep reorganization detected after buffer is full
        let mut log_buffer = LogBuffer::new(); // Reset buffer
        for i in 1..=BUFFER_SIZE {
            log_buffer.add_block(&mock_block_header(i as u64));
        }
        //triggers deep reorganization
        log_buffer.add_block(&mock_block_header(BUFFER_SIZE as u64 + 100));
        assert!(log_buffer.detect_deep_reorganization().is_err(), "Deep reorganization not detected after buffer is full");
    }

    use web3::{transports::Http, Web3, types::{FilterBuilder, BlockNumber}};
    use std::str::FromStr;
    
    #[tokio::test]
    async fn fetch_log_and_parse() -> Result<(), Box<dyn std::error::Error>> {
        // Set up web3 with the HTTP transport
        let http = Http::new("https://mainnet.infura.io/v3/f5373e503b134ffdb9a00d30f4c22bb1")?;
        let web3 = Web3::new(http);
        let contract_address = web3::types::H160::from_slice(
            &hex::decode("5777d92f208679db4b9778590fa3cab3ac9e2168").unwrap()[..],
        );
        let contract = web3::contract::Contract::from_json(
            web3.eth(),
            contract_address,
            include_bytes!("contracts/uniswap_pool_abi.json"),
        )?;
        let block_hash = H256::from_str("0xe7169e50c0ebeccd268ee16defc771b302af2bd4422bb3eae5b29626b9e56eab")?;
        let swap_event=  contract.abi().events_by_name("Swap")?.first().unwrap();
        let filter =  web3::types::FilterBuilder::default()
        .block_hash(block_hash)
        .address(vec![contract_address])
        .topics(Some(vec![swap_event.signature()]), None, None, None)
        .build();
    
        let logs = web3.eth().logs(filter).await?;
               
        if let Some(log) = logs.first() {
            let swap_log = SwapLog::from_log(&swap_event, log)?;
            SwapLog::print_details(&swap_log);
                // Check if the amounts are within the acceptable delta
            assert_eq!(swap_log.sender, "0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad".parse()?);
            assert_eq!(swap_log.recipient, "0x8fb892e9c203752dcd4ce5423263b329baf070b7".parse()?);            
            assert!((swap_log.decimal0 + (227732.60000325253)) == 0.0);
            assert!((swap_log.decimal1 - 227754.40344).abs() == 0.0);
        } else {
            return Err("No logs found".into());
        }
      
    
        Ok(())
    }
}