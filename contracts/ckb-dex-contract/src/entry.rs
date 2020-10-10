// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
  ckb_constants::Source,
  ckb_types::{bytes::Bytes, prelude::*},
  debug, default_alloc,
  dynamic_loading::CKBDLContext,
  high_level::{
    load_cell_capacity, load_cell_data, load_script, load_transaction, load_witness_args,
  },
};

use blake2b_ref::{Blake2b, Blake2bBuilder};
use ckb_lib_secp256k1::LibSecp256k1;

use crate::error::Error;

// Alloc 4K fast HEAP + 2M HEAP to receives PrefilledData
default_alloc!(4 * 1024, 2048 * 1024, 64);

const FEE: f32 = 0.003;
const ORDER_LEN: usize = 41;
const SUDT_LEN: usize = 16;
const PRICE_PARAM: f32 = 100000.0;

fn new_blake2b() -> Blake2b {
  Blake2bBuilder::new(32)
    .personal(b"ckb-default-hash")
    .build()
}

fn validate_signature() -> Result<(), Error> {
  let script = load_script()?;
  let args: Bytes = script.args().unpack();

  if args.len() != 20 {
    return Err(Error::Encoding);
  }

  let witness_args = load_witness_args(0, Source::GroupInput)?;

  // create a DL context with 128K buffer size
  let mut context = CKBDLContext::<[u8; 128 * 1024]>::new();
  let lib = LibSecp256k1::load(&mut context);

  let witness: Bytes = witness_args
    .input_type()
    .to_opt()
    .ok_or(Error::Encoding)?
    .unpack();
  let mut message = [0u8; 32];
  let mut signature = [0u8; 65];
  let msg_len = message.len();
  let sig_len = signature.len();
  assert_eq!(witness.len(), message.len() + signature.len());
  message.copy_from_slice(&witness[..msg_len]);
  signature.copy_from_slice(&witness[msg_len..msg_len + sig_len]);
  // recover pubkey_hash
  let prefilled_data = lib.load_prefilled_data().map_err(|err| {
    debug!("load prefilled data error: {}", err);
    Error::LoadPrefilledData
  })?;
  let pubkey = lib
    .recover_pubkey(&prefilled_data, &signature, &message)
    .map_err(|err| {
      debug!("recover pubkey error: {}", err);
      Error::RecoverPubkey
    })?;
  let pubkey_hash = {
    let mut buf = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(pubkey.as_slice());
    hasher.finalize(&mut buf);
    buf
  };
  if &args[..] != &pubkey_hash[..20] {
    return Err(Error::WrongPubkey);
  }

  Ok(())
}

fn parse_order_data(data: &[u8]) -> Result<(u128, u128, u64, u8), Error> {
  debug!("data is {:?}", data);
  // dealt(u128) or dealt(u128) + undealt(u128) + price(u64) + order_type(u8)
  if data.len() != SUDT_LEN || data.len() != ORDER_LEN {
    return Err(Error::WrongDataLengthOrFormat);
  }
  let mut dealt_amount_buf = [0u8; 16];
  let mut undealt_amount_buf = [0u8; 16];
  let mut price_buf = [0u8; 8];
  let mut order_type_buf = [0u8; 1];

  dealt_amount_buf.copy_from_slice(&data[0..16]);
  if data.len() == 41 {
    undealt_amount_buf.copy_from_slice(&data[16..32]);
    price_buf.copy_from_slice(&data[32..40]);
    order_type_buf.copy_from_slice(&data[40..41]);
  }
  Ok((
    u128::from_be_bytes(dealt_amount_buf),
    u128::from_be_bytes(undealt_amount_buf),
    u64::from_be_bytes(price_buf),
    u8::from_be_bytes(order_type_buf),
  ))
}

fn validate_order() -> Result<(), Error> {
  let script = load_script()?;
  let args: Bytes = script.args().unpack();
  let tx = load_transaction().unwrap().raw();

  let mut input_capacity = 0;
  let mut output_capacity = 0;
  let mut input_data_buf = [0u8; ORDER_LEN];
  let mut output_data_buf = [0u8; ORDER_LEN];
  let len = tx.outputs().len();
  let mut index = 0;
  while index < len {
    let output_lock_args = tx.outputs().get(index).unwrap().lock().args().as_bytes();
    if &output_lock_args[0..20] == &args[0..20] {
      input_capacity = load_cell_capacity(index, Source::Input).unwrap();
      output_capacity = load_cell_capacity(index, Source::Output).unwrap();

      let mut data = load_cell_data(index, Source::Input).unwrap();
      input_data_buf.copy_from_slice(&data);
      data = load_cell_data(index, Source::Output).unwrap();
      output_data_buf.copy_from_slice(&data);
      break;
    }
    index += 1;
  }

  let (input_dealt_amount, _, price, order_type) = parse_order_data(&input_data_buf)?;
  let (output_dealt_amount, _, _, _) = parse_order_data(&output_data_buf)?;

  let order_price: f32 = (price as f32) / PRICE_PARAM;

  let diff_capacity: f32;
  let diff_sudt_amount: f32;

  // Buy SUDT
  if order_type == 0 {
    if input_capacity < output_capacity || input_dealt_amount > output_dealt_amount {
      return Err(Error::WrongSUDTAmount);
    }
    diff_capacity = (input_capacity - output_capacity) as f32;
    diff_sudt_amount = (output_dealt_amount - input_dealt_amount) as f32;
  } else if order_type == 1 {
    // Sell SUDT
    if input_capacity > output_capacity || input_dealt_amount < output_dealt_amount {
      return Err(Error::WrongSUDTAmount);
    }
    diff_capacity = (output_capacity - input_capacity) as f32;
    diff_sudt_amount = (input_dealt_amount - output_dealt_amount) as f32;
  } else {
    return Err(Error::WrongOrderType);
  }

  if diff_sudt_amount < diff_capacity / (1.0 + FEE) / order_price {
    return Err(Error::WrongSUDTAmount);
  }

  Ok(())
}

pub fn main() -> Result<(), Error> {
  let witness_args = load_witness_args(0, Source::GroupInput)?;

  if witness_args.input_type().to_opt().is_none() {
    return validate_order();
  } else {
    return validate_signature();
  }
}
