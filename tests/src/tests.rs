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
            inputs_data.get(index).unwrap().clone(),
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
// Assume the sudt decimal is 8 and the price 5 sudt/ckb
fn test_ckb_sudt_partial_order() {
    // inputs data
    // input1: dealt_amount(50sudt) + undealt_amount(150sudt) + price(5*10^10) + buy(00)
    let mut dealt_amount = 0x12A05F200u128.to_le_bytes().to_vec();
    let mut undealt_amount = 0x37E11D600u128.to_le_bytes().to_vec();
    let mut price = 0xBA43B7400u64.to_le_bytes().to_vec();
    let mut order_type = vec![0x00];
    dealt_amount.append(&mut undealt_amount);
    dealt_amount.append(&mut price);
    dealt_amount.append(&mut order_type);
    let input1_data: &[u8] = &dealt_amount;

    // input2: dealt_amount(100sudt) + undealt_amount(200sudt) + price(5*10^10) + sell(01)
    dealt_amount = 0x2540BE400u128.to_le_bytes().to_vec();
    undealt_amount = 0x4A817C800u128.to_le_bytes().to_vec();
    price = 0xBA43B7400u64.to_le_bytes().to_vec();
    order_type = vec![0x01];
    dealt_amount.append(&mut undealt_amount);
    dealt_amount.append(&mut price);
    dealt_amount.append(&mut order_type);
    let input2_data: &[u8] = &dealt_amount;

    let inputs_data = vec![input1_data, input2_data];

    // outputs data
    // output1: dealt_amount(250sudt)
    let mut dealt_amount = 0x5D21DBA00u128.to_le_bytes().to_vec();
    let output1_data: &[u8] = &dealt_amount;

    // output2: dealt_amount(250sudt) + undealt_amount(50sudt) + price(5*10^10) + sell(01)
    dealt_amount = 0x5D21DBA00u128.to_le_bytes().to_vec();
    undealt_amount = 0x12A05F200u128.to_le_bytes().to_vec();
    price = 0xBA43B7400u64.to_le_bytes().to_vec();
    order_type = vec![0x01];
    dealt_amount.append(&mut undealt_amount);
    dealt_amount.append(&mut price);
    dealt_amount.append(&mut order_type);
    let output2_data: &[u8] = &dealt_amount;

    let outputs_data = vec![output1_data, output2_data];

    let inputs_args = vec![
        Bytes::from("7e7a30e75685e4d332f69220e925575dd9b84676"),
        Bytes::from("a53ce751e2adb698ca10f8c1b8ebbee20d41a842"),
    ];
    let outputs_args = vec![
        Bytes::from("7e7a30e75685e4d332f69220e925575dd9b84676"),
        Bytes::from("a53ce751e2adb698ca10f8c1b8ebbee20d41a842"),
    ];
    let (mut context, tx) = build_test_context(
        vec![1000, 500],
        vec![700, 800],
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
