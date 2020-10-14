use super::*;
use ckb_testtool::context::Context;
use ckb_tool::ckb_types::{
    bytes::Bytes,
    core::{Capacity, TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};

const MAX_CYCLES: u64 = 1000_0000;

fn build_test_context(
    inputs_token: Vec<u64>,
    outputs_token: Vec<u64>,
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
    // let secp256k1_bin: Bytes = Loader::default().load_binary("secp256k1_blake2b_sighash_all_dual");
    // let secp256k1_out_point = context.deploy_cell(secp256k1_bin);

    // // build lock script
    // let secp256k1_lock_script_dep = CellDep::new_builder()
    //     .out_point(secp256k1_out_point)
    //     .build();

    // prepare inputs
    let mut inputs = vec![];
    for index in 0..inputs_token.len() {
        let dex_script = context
            .build_script(&dex_out_point, input_args.get(index).unwrap().clone())
            .expect("script");
        let token = inputs_token.get(index).unwrap();
        let capacity = Capacity::shannons(*token);
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
    for index in 0..outputs_token.len() {
        let dex_script = context
            .build_script(&dex_out_point, output_args.get(index).unwrap().clone())
            .expect("script");
        let token = outputs_token.get(index).unwrap();
        let capacity = Capacity::shannons(*token);
        let output = CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(dex_script.clone())
            .build();
        outputs.push(output);
    }

    let dex_script_dep = CellDep::new_builder().out_point(dex_out_point).build();

    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        // .cell_dep(secp256k1_lock_script_dep)
        .cell_dep(dex_script_dep)
        .build();
    (context, tx)
}

#[test]
// Assume the sudt decimal is 8 and the price 5 sudt/ckb
fn test_ckb_sudt_partial_order1() {
    // input1: dealt_amount(50sudt 0x12A05F200u128) + undealt_amount(150sudt 0x37E11D600u128) + price(5*10^10 0xBA43B7400u64) + buy(00)
    // input2: dealt_amount(100sudt 0x2540BE400u128) + undealt_amount(200sudt 0x4A817C800u128) + price(5*10^10 0xBA43B7400u64) + sell(01)
    let inputs_data = vec![
        Bytes::from(
            hex::decode("00F2052A01000000000000000000000000D6117E03000000000000000000000000743BA40B00000000").unwrap(),
        ),
        Bytes::from(
            hex::decode("00E40B5402000000000000000000000000C817A804000000000000000000000000743BA40B00000001").unwrap(),
        ),
    ];

    // output1: dealt_amount(200sudt 0x4A817C800u128)
    // output2: dealt_amount(250sudt 0x5CF6F14C0u64)
    // + undealt_amount(49.55sudt 0x12A05F200u128) + price(5*10^10 0xBA43B7400u64) + sell(01)
    let outputs_data = vec![
        Bytes::from(hex::decode("00C817A8040000000000000000000000").unwrap()),
        Bytes::from(
            hex::decode("C0146FCF05000000000000000000000000F2052A01000000000000000000000000743BA40B00000001").unwrap(),
        ),
    ];

    let inputs_args = vec![
        Bytes::from(hex::decode("7e7a30e75685e4d332f69220e925575dd9b84676").unwrap()),
        Bytes::from(hex::decode("a53ce751e2adb698ca10f8c1b8ebbee20d41a842").unwrap()),
    ];
    let outputs_args = vec![
        Bytes::from(hex::decode("7e7a30e75685e4d332f69220e925575dd9b84676").unwrap()),
        Bytes::from(hex::decode("a53ce751e2adb698ca10f8c1b8ebbee20d41a842").unwrap()),
    ];
    // output1 capacity = 2000 - 750 * (1 + 0.003) = 1247.75
    let (mut context, tx) = build_test_context(
        vec![200000000000, 80000000000],
        vec![124775000000, 155000000000],
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
fn test_ckb_sudt_all_order1() {
    // input1: dealt_amount(50sudt 0x12A05F200u128) + undealt_amount(150sudt 0x37E11D600u128) + price(5*10^10 0xBA43B7400u64) + buy(00)
    // input2: dealt_amount(100sudt 0x2540BE400u128) + undealt_amount(150.45sudt 0x380C07B40u128) + price(5*10^10 0xBA43B7400u64) + sell(01)
    let inputs_data = vec![
        Bytes::from(
            hex::decode("00F2052A01000000000000000000000000D6117E03000000000000000000000000743BA40B00000000").unwrap(),
        ),
        Bytes::from(
            hex::decode("00E40B54020000000000000000000000407BC08003000000000000000000000000743BA40B00000001").unwrap(),
        ),
    ];

    // output1: dealt_amount(200sudt 0x5D21DBA00u128)
    // output2: 0x0
    let outputs_data = vec![
        Bytes::from(hex::decode("00C817A8040000000000000000000000").unwrap()),
        Bytes::new(),
    ];

    let inputs_args = vec![
        Bytes::from(hex::decode("7e7a30e75685e4d332f69220e925575dd9b84676").unwrap()),
        Bytes::from(hex::decode("a53ce751e2adb698ca10f8c1b8ebbee20d41a842").unwrap()),
    ];
    let outputs_args = vec![
        Bytes::from(hex::decode("7e7a30e75685e4d332f69220e925575dd9b84676").unwrap()),
        Bytes::from(hex::decode("a53ce751e2adb698ca10f8c1b8ebbee20d41a842").unwrap()),
    ];
    // output1 capacity = 2000 - 750 * (1 + 0.003) = 1247.75
    let (mut context, tx) = build_test_context(
        vec![200000000000, 80000000000],
        vec![124775000000, 155000000000],
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
fn test_ckb_sudt_all_order2() {
    // input1: dealt_amount(0sudt 0x0u128) + undealt_amount(150sudt 0x37E11D600u128) + price(5*10^10 0xBA43B7400u64) + buy(00)
    // input2: dealt_amount(0sudt 0x0u128) + undealt_amount(150.45sudt 0x380C07B40u128) + price(5*10^10 0xBA43B7400u64) + sell(01)
    let inputs_data = vec![
        Bytes::from(
            hex::decode("0000000000000000000000000000000000D6117E03000000000000000000000000743BA40B00000000").unwrap(),
        ),
        Bytes::from(
            hex::decode("00000000000000000000000000000000407BC08003000000000000000000000000743BA40B00000001").unwrap(),
        ),
    ];

    // output1: dealt_amount(200sudt 0x5D21DBA00u128)
    // output2: 0x0
    let outputs_data = vec![
        Bytes::from(hex::decode("00C817A8040000000000000000000000").unwrap()),
        Bytes::new(),
    ];

    let inputs_args = vec![
        Bytes::from(hex::decode("7e7a30e75685e4d332f69220e925575dd9b84676").unwrap()),
        Bytes::from(hex::decode("a53ce751e2adb698ca10f8c1b8ebbee20d41a842").unwrap()),
    ];
    let outputs_args = vec![
        Bytes::from(hex::decode("7e7a30e75685e4d332f69220e925575dd9b84676").unwrap()),
        Bytes::from(hex::decode("a53ce751e2adb698ca10f8c1b8ebbee20d41a842").unwrap()),
    ];
    // output1 capacity = 2000 - 750 * (1 + 0.003) = 1247.75
    // output2 capacity = 800 + 750 = 1550
    let (mut context, tx) = build_test_context(
        vec![200000000000, 80000000000],
        vec![124775000000, 155000000000],
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