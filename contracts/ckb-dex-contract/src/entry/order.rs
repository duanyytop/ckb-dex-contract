// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
  ckb_constants::Source,
  ckb_types::{bytes::Bytes, prelude::*},
  error::SysError,
  high_level::{
    load_cell_capacity, load_cell_data, load_script, load_transaction
  },
};

use crate::error::Error;

const FEE: f64 = 0.003;
const ORDER_LEN: usize = 57;
const SUDT_LEN: usize = 16;
// real price * 10 ^ 10 = cell price data
const PRICE_PARAM: f64 = 10000000000.0;

struct OrderData {
  sudt_amount: u128,
  dealt_amount: u128,
  undealt_amount: u128,
  price: u64,
  order_type: u8,
}

fn _init_order_data() -> OrderData {
  OrderData {
    sudt_amount: 0u128,
    dealt_amount: 0u128,
    undealt_amount: 0u128,
    price: 0u64,
    order_type: 0u8,
  }
}


fn parse_order_data(data: &[u8]) -> Result<OrderData, Error> {
  // sudt_amount(u128) or sudt_amount(u128) + dealt(u128) + undealt(u128) + price(u64) + order_type(u8)
  if data.len() != SUDT_LEN && data.len() != ORDER_LEN {
    return Err(Error::WrongDataLengthOrFormat);
  }
  let mut sudt_amount_buf = [0u8; 16];
  let mut dealt_amount_buf = [0u8; 16];
  let mut undealt_amount_buf = [0u8; 16];
  let mut price_buf = [0u8; 8];
  let mut order_type_buf = [0u8; 1];

  sudt_amount_buf.copy_from_slice(&data[0..16]);
  if data.len() == ORDER_LEN {
    dealt_amount_buf.copy_from_slice(&data[16..32]);
    undealt_amount_buf.copy_from_slice(&data[32..48]);
    price_buf.copy_from_slice(&data[48..56]);
    order_type_buf.copy_from_slice(&data[56..57]);
  }
  Ok(OrderData {
    sudt_amount: u128::from_le_bytes(sudt_amount_buf),
    dealt_amount: u128::from_le_bytes(dealt_amount_buf),
    undealt_amount: u128::from_le_bytes(undealt_amount_buf),
    price: u64::from_le_bytes(price_buf),
    order_type: u8::from_le_bytes(order_type_buf),
  })
}

fn parse_cell_data(index: usize, source: Source) -> Result<OrderData, Error> {
  let data = match load_cell_data(index, source) {
      Ok(data) => data,
      Err(SysError::IndexOutOfBound) => return Err(Error::IndexOutOfBound),
      Err(err) => return Err(err.into()),
  };
  let order_data = match data.len() {
    ORDER_LEN => {
      let mut data_buf = [0u8; ORDER_LEN];
      data_buf.copy_from_slice(&data);
      parse_order_data(&data_buf)?
    }
    SUDT_LEN => {
      let mut data_buf = [0u8; SUDT_LEN];
      data_buf.copy_from_slice(&data);
      parse_order_data(&data_buf)?
    }
    _ => _init_order_data(),
  };
  Ok(order_data)
}

pub fn validate() -> Result<(), Error> {
  let script = load_script()?;
  let args: Bytes = script.args().unpack();
  let tx = match load_transaction() {
    Ok(tx) => tx.raw(),
    Err(err) => return Err(err.into()),
  };

  if tx.inputs().len() != tx.outputs().len() {
    return Err(Error::InputsAndOutputsAmountNotSame);
  }

  let mut input_capacity = 0u64;
  let mut output_capacity = 0u64;
  let mut input_order: OrderData = _init_order_data();
  let mut output_order: OrderData = _init_order_data();
  for index in 0..tx.outputs().len() {
    let output_lock_args: Bytes = match tx.outputs().get(index) {
      Some(output) => output.lock().args().unpack(),
      None => return Err(Error::IndexOutOfBound),
    };
    if &output_lock_args[0..20] == &args[0..20] {
      input_capacity = load_cell_capacity(index, Source::Input)?;
      output_capacity = load_cell_capacity(index, Source::Output)?;
      input_order = parse_cell_data(index, Source::Input)?;
      output_order = parse_cell_data(index, Source::Output)?;
      break;
    }
  }

  // debug!("input dealt and undealt amount: {}, {}", input_order.dealt_amount, input_order.undealt_amount);
  // debug!("output dealt and undealt amount: {}, {}", output_order.dealt_amount, output_order.undealt_amount);
  // debug!("input and output capacity: {:?}, {:?}", input_capacity, output_capacity);

  if input_order.undealt_amount == 0 {
    return Err(Error::WrongSUDTInputAmount);
  }
  if input_order.price == 0 {
    return Err(Error::OrderPriceNotZero);
  }
  let order_price: f64 = input_order.price as f64 / PRICE_PARAM;
 
  // Buy SUDT
  if input_order.order_type == 0 {
    if input_capacity < output_capacity {
      return Err(Error::WrongDiffCapacity);
    }
    if input_order.sudt_amount > output_order.sudt_amount || input_order.undealt_amount < output_order.undealt_amount  {
      return Err(Error::WrongSUDTDiffAmount);
    }

    let diff_undealt_amount = (input_order.undealt_amount - output_order.undealt_amount) as f64;

    if output_order.dealt_amount != 0 && output_order.undealt_amount != 0 {
      if input_order.dealt_amount > output_order.dealt_amount {
        return Err(Error::WrongSUDTDiffAmount);
      }
      let diff_dealt_amount = (output_order.dealt_amount - input_order.dealt_amount) as f64;

      if diff_dealt_amount != diff_undealt_amount {
        return Err(Error::WrongSUDTDiffAmount);
      }
    }

    let diff_capacity = (input_capacity - output_capacity) as f64;
    let diff_sudt_amount = (output_order.sudt_amount - input_order.sudt_amount) as f64;
    
    if diff_sudt_amount != diff_undealt_amount {
      return Err(Error::WrongSUDTDiffAmount);
    }

    // Floating point numbers have precision errors
    if diff_undealt_amount - diff_capacity / (1.0 + FEE) / order_price > 0.001{
      return Err(Error::WrongSwapAmount);
    }
  } else if input_order.order_type == 1 {
    // Sell SUDT
    if input_capacity > output_capacity {
      return Err(Error::WrongDiffCapacity);
    }

    if input_order.sudt_amount < output_order.sudt_amount || input_order.undealt_amount < output_order.undealt_amount {
      return Err(Error::WrongSUDTDiffAmount);
    }

    let diff_undealt_amount = (input_order.undealt_amount - output_order.undealt_amount) as f64;

    if output_order.dealt_amount != 0 || output_order.undealt_amount != 0 {
      if input_order.dealt_amount > output_order.dealt_amount {
        return Err(Error::WrongSUDTDiffAmount);
      }
      let diff_dealt_amount = (output_order.dealt_amount - input_order.dealt_amount) as f64;

      if diff_dealt_amount != diff_undealt_amount {
        return Err(Error::WrongSUDTDiffAmount);
      }
    }

    let diff_capacity = (output_capacity - input_capacity) as f64;
    let diff_sudt_amount = (input_order.sudt_amount - output_order.sudt_amount) as f64;
    
    // Floating point numbers have precision errors
    if diff_sudt_amount - diff_undealt_amount * (1.0 + FEE) > 0.001 {
      return Err(Error::WrongSUDTDiffAmount);
    }

    if diff_capacity < diff_sudt_amount / (1.0 + FEE) / order_price {
      return Err(Error::WrongSwapAmount);
    }
  } else {
    return Err(Error::WrongOrderType);
  }

  Ok(())
}
