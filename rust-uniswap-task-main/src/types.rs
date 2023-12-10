use web3::{
    types::{BlockHeader, U64, Log, U256,H160},
	ethabi::{self, Event},
};
use std::collections::VecDeque;
use anyhow::Error;

pub struct BlockWithLogs {
    block: BlockHeader,
    logs: Vec<Log>,
}
pub struct SwapLog {
    sender: H160,
    recipient: H160,
    amount0: U256,
    amount1: U256,
	decimal0: f64,
	decimal1: f64,	
}
pub struct LogBuffer {
    buffer: VecDeque<BlockWithLogs>,
}

const BUFFER_SIZE: usize = 6; 

impl BlockWithLogs {
    pub fn new(block: BlockHeader, logs: Vec<Log>) -> Self {
        BlockWithLogs { block, logs }
    }
}
impl SwapLog {
    // Function to create a SwapLog from a raw Ethereum log
    fn from_log(swap_event_abi: &Event, log: &Log) -> Result<Self, Error> {
        // Ensure the log has the correct number of parameters
       /*if swap_event_abi.inputs.len() != 4 {
            return Err(Error::msg("Invalid number of parameters in swap event"));
        }*/
	
	 // Decode the log
	 let parsed_log = swap_event_abi.parse_log(ethabi::RawLog {
        topics: log.topics.clone(),
        data: log.data.0.clone(),
    })?;
    println!("{:?}", parsed_log);
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

    pub fn add_block(&mut self, block_with_logs: BlockWithLogs) {
        self.buffer.push_back(block_with_logs);
        println!("Block added to buffer, buffer size:{:?}",  self.buffer.len());
        
    }

    pub fn process(&mut self, swap_event: &Event) -> Result<(), anyhow::Error> {
        if let Some(latest_block_with_logs) = self.buffer.back() {
            let latest_block_number = latest_block_with_logs.block.number.unwrap();

            if Self::detect_deep_reorganization(&self.buffer, latest_block_number) {
                println!("Deep reorganization detected. Exiting.");
                std::process::exit(1);
            }
        }
        // process the buffer when full
        if self.buffer.len() == BUFFER_SIZE {
            println!("Processing logs.");
            while let Some(block_with_logs) = self.buffer.pop_front() {
                for log in block_with_logs.logs {
                    if let Ok(swap_log) = SwapLog::from_log(swap_event, &log) {
                        swap_log.print_details();
                    }
                }
            }
         }

    

        Ok(())
    }
    fn detect_deep_reorganization(buffer: &VecDeque<BlockWithLogs>, current_block_number: U64) -> bool {
        if let Some(confirmed_block) = buffer.front() {
            if confirmed_block.block.number.unwrap() + U64::from(BUFFER_SIZE as u64) <= current_block_number {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use serde_json::Value;
	struct BlockHeader {
		number: U64,
		
	}
	
	struct BlockWithLogs {
		block: BlockHeader,
		logs: Vec<Log>,
	}
    #[test]
    fn test_deep_reorganization_detection() {
        let mut file = File::open("resources/mocked_chain.json").expect("file not found");
        let mut data = String::new();
        file.read_to_string(&mut data).expect("error reading file");
        let blocks: Vec<Value> = serde_json::from_str(&data).expect("error parsing json");
		let mut buffer = VecDeque::<BlockWithLogs>::new();
        const BUFFER_SIZE: usize = 6;
        let mut deep_reorg_detected = false;
        // Simulate block stream from JSON data
		
        for block in blocks {
			let block_number = U64::from(block["number"].as_u64().expect("block number missing"));;
			// Create an instance of BlockWithLogs
			let block_header = BlockHeader {
				number: block_number,
			};
			let logs = Vec::<Log>::new();
			let block_with_logs = BlockWithLogs {
				block: block_header,
				logs,
			};
			buffer.push_back(BlockWithLogs::from(block_with_logs));
            if buffer.len() > BUFFER_SIZE {
				deep_reorg_detected = detect_deep_reorganization(&buffer,block_number)
			} 
        }

        // If no deep reorg was detected, the test should fail
        assert!(!deep_reorg_detected, "Deep reorganization detected");
    }
}