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
        if self.buffer.len() >= BUFFER_SIZE {
        if let Some(latest_block_with_logs) = self.buffer.back() {
            let latest_block_number = latest_block_with_logs.block.number.unwrap();
    
            if Self::detect_deep_reorganization(&self.buffer, latest_block_number) {
                println!("Deep reorganization detected. Clearing buffer and skipping processing.");
                std::process::exit(1);
            } else {
                for log in latest_block_with_logs.logs.clone().into_iter() {
                    if let Err(err) = SwapLog::from_log(swap_event, &log) {
                        // Log the error and skip processing the block
                        println!("Error processing block: {}", err);
                        break;
                    } else if let Ok(swap_log) = SwapLog::from_log(swap_event,&log) {
                        swap_log.print_details();

                    }
            }
            self.buffer.pop_front(); 
    
          
            }
        } 
         } else {
            println!("Buffer is not full. Skipping processing.");
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

