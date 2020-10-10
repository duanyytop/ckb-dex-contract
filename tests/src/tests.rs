use super::*;
use ckb_testtool::context::Context;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{Capacity, TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_tool::{ckb_error::assert_error_eq, ckb_script::ScriptError};

const MAX_CYCLES: u64 = 100_0000;

// errors
const ERROR_AMOUNT: i8 = 5;

fn build_test_context(
    inputs_token: Vec<usize>,
    outputs_token: Vec<usize>,
    inputs_data: Vec<Bytes>,
    outputs_data: Vec<Bytes>,
    input_args: Vec<Bytes>,
    output_args: Vec<Bytes>,
) -> (Context, TransactionView) {
    // deploy dex script
    let mut context = Context::default();
    let dex_bin: Bytes = Loader::default().load_binary("ckb-dex-contract");
    let dex_out_point = context.deploy_cell(dex_bin);

    // deploy secp256k1_blake2b_sighash_all script
    let secp256k1_bin: Bytes = Loader::default().load_binary("secp256k1_blake2b_sighash_all_dual");
    let secp256k1_out_point = context.deploy_cell(secp256k1_bin);

    // build lock script
    let secp256k1_lock_script_dep = CellDep::new_builder()
        .out_point(secp256k1_out_point)
        .build();

    // prepare inputs
    let mut inputs = vec![];
    for index in 0..input_args.len() {
        let dex_script = context
            .build_script(&dex_out_point, input_args.get(index).unwrap().clone())
            .expect("script");
        let token = inputs_token.get(index).unwrap();
        let capacity = Capacity::bytes(token.clone()).unwrap().as_u64();
        let input_out_point = context.create_cell(
            CellOutput::new_builder()
                .capacity(capacity.pack())
                .lock(dex_script.clone())
                .build(),
            token.to_le_bytes().to_vec().into(),
        );
        let input = CellInput::new_builder()
            .previous_output(input_out_point)
            .build();
        inputs.push(input);
    }

    // prepare outputs
    let mut outputs = vec![];
    for index in 0..output_args.len() {
        let dex_script = context
            .build_script(&dex_out_point, output_args.get(index).unwrap().clone())
            .expect("script");
        let token = outputs_token.get(index).unwrap();
        let capacity = Capacity::bytes(token.clone()).unwrap().as_u64();
        let output = CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(dex_script.clone())
            .build();
        outputs.push(output);
    }

    let outputs_data_vec: Vec<_> = outputs_data
        .iter()
        .map(|token| Bytes::from(token.to_vec()))
        .collect();

    let dex_script_dep = CellDep::new_builder().out_point(dex_out_point).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data_vec.pack())
        .cell_dep(secp256k1_lock_script_dep)
        .cell_dep(dex_script_dep)
        .build();
    (context, tx)
}

#[test]
fn test_basic() {
    let inputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let inputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let (mut context, tx) = build_test_context(
        vec![1000],
        vec![400, 600],
        inputs_data,
        outputs_data,
        inputs_args,
        outputs_args,
    );
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("cycles: {}", cycles);
}

#[test]
fn test_destroy_udt() {
    let inputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let inputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let (mut context, tx) = build_test_context(
        vec![1000],
        vec![800, 100, 50],
        inputs_data,
        outputs_data,
        inputs_args,
        outputs_args,
    );
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("cycles: {}", cycles);
}

#[test]
fn test_create_sudt_without_owner_mode() {
    let inputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let inputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let (mut context, tx) = build_test_context(
        vec![1000],
        vec![1200],
        inputs_data,
        outputs_data,
        inputs_args,
        outputs_args,
    );
    let tx = context.complete_tx(tx);

    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_error_eq!(err, ScriptError::ValidationFailure(ERROR_AMOUNT));
}

#[test]
fn test_create_sudt_with_owner_mode() {
    let inputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_data = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let inputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let outputs_args = vec![Bytes::from("2a45"), Bytes::from("2a45")];
    let (mut context, tx) = build_test_context(
        vec![1000],
        vec![1200],
        inputs_data,
        outputs_data,
        inputs_args,
        outputs_args,
    );
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("cycles: {}", cycles);
}
