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
  high_level::{load_cell_data, load_script, load_witness_args},
};

use blake2b_ref::{Blake2b, Blake2bBuilder};
use ckb_lib_secp256k1::LibSecp256k1;

use crate::error::Error;

// Alloc 4K fast HEAP + 2M HEAP to receives PrefilledData
default_alloc!(4 * 1024, 2048 * 1024, 64);

const FEE: f32 = 0.003;

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

fn parse_order_data(data: &str) -> Result<(u128, u128, u64, u8), Error> {
  debug!("data is {:?}", data);
  // dealt(u128) + undealt(u128) + price(u64) + order_type(u8)
  if data.len() < 41 {
    return Err(Error::DataFieldIsTooSmall);
  }
  let mut dealt_amount_buf = [0u8; 16];
  let mut undealt_amount_buf = [0u8; 16];
  let mut price_buf = [0u8; 8];
  let mut order_type_buf = [0u8; 1];
  dealt_amount_buf.copy_from_slice(&data[0..16]);
  undealt_amount_buf.copy_from_slice(&data[16..32]);
  price_buf.copy_from_slice(&data[32..40]);
  order_type_buf.copy_from_slice(&data[40..41]);
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
  let mut output_capacity = 0;

  let tx = load_transaction().unwrap();

  let mut input_capacity = String::from("0");
  let mut input_order_data = String::from("0");
  let len = tx.outputs().len();
  for index in [0..len] {
    if tx.inputs()[index].lock().args().unpack() == args {
      input_capacity = u64::from_be_bytes(tx.inputs()[index].capacity().unpack());
      input_order_data = tx.outputs_data()[index].unpack();
      break;
    }
  }

  let mut output_capacity = String::from("0");
  let mut output_order_data = String::from("0");
  let len = tx.outputs().len();
  for index in [0..len] {
    if tx.outputs()[index].lock().args().unpack() == args {
      output_capacity = u64::from_be_bytes(tx.outputs()[index].capacity().unpack());
      output_order_data = tx.outputs_data()[index].unpack();
      break;
    }
  }

  let (input_dealt_amount, _, price, order_type) = parse_order_data(input_order_data);
  let (output_dealt_amount, _, _, _) = parse_order_data(output_order_data);

  if output_dealt_amount - input_dealt_amount
    < (output_capacity - input_capacity) / (1 + FEE) / price
  {
    return Err(Error::SUDTAmount);
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
