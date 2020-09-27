// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use ckb_std::{
  ckb_constants::Source,
  ckb_types::{bytes::Bytes, prelude::*},
  high_level::{load_cell_data, load_cell_lock_hash, load_script, QueryIter},
};

use crate::error::Error;

const UDT_LEN: usize = 16;

fn check_owner_mode(args: &Bytes) -> Result<bool, Error> {
  // With owner lock script extracted, we will look through each input in the
  // current transaction to see if any unlocked cell uses owner lock.
  let is_owner_mode = QueryIter::new(load_cell_lock_hash, Source::Input)
    .find(|lock_hash| args[..] == lock_hash[..])
    .is_some();
  Ok(is_owner_mode)
}

fn collect_inputs_amount() -> Result<u128, Error> {
  // let's loop through all input cells containing current UDTs,
  // and gather the sum of all input tokens.
  let mut buf = [0u8; UDT_LEN];

  let udt_list = QueryIter::new(load_cell_data, Source::GroupInput)
    .map(|data| {
      if data.len() == UDT_LEN {
        buf.copy_from_slice(&data);
        // u128 is 16 bytes
        Ok(u128::from_le_bytes(buf))
      } else {
        Err(Error::Encoding)
      }
    })
    .collect::<Result<Vec<_>, Error>>()?;
  Ok(udt_list.into_iter().sum::<u128>())
}

fn collect_outputs_amount() -> Result<u128, Error> {
  // With the sum of all input UDT tokens gathered, let's now iterate through
  // output cells to grab the sum of all output UDT tokens.
  let mut buf = [0u8; UDT_LEN];

  let udt_list = QueryIter::new(load_cell_data, Source::GroupOutput)
    .map(|data| {
      if data.len() == UDT_LEN {
        buf.copy_from_slice(&data);
        // u128 is 16 bytes
        Ok(u128::from_le_bytes(buf))
      } else {
        Err(Error::Encoding)
      }
    })
    .collect::<Result<Vec<_>, Error>>()?;
  Ok(udt_list.into_iter().sum::<u128>())
}

pub fn main() -> Result<(), Error> {
  // remove below examples and write your code here

  let script = load_script()?;
  let args: Bytes = script.args().unpack();

  // return success if owner mode is true
  if check_owner_mode(&args)? {
    return Ok(());
  }

  let inputs_amount = collect_inputs_amount()?;
  let outputs_amount = collect_outputs_amount()?;

  if inputs_amount < outputs_amount {
    return Err(Error::Amount);
  }

  Ok(())
}
