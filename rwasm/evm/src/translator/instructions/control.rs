use crate::{
    translator::{
        gas,
        host::Host,
        instruction_result::InstructionResult,
        instructions::{
            opcode::{compute_push_count, JUMP, JUMPI, PUSH0, PUSH32},
            utilities::replace_with_call_to_subroutine,
        },
        translator::Translator,
    },
    utilities::{
        invalid_op_gen,
        load_i64_const,
        sp_drop_u256,
        sp_get_offset,
        stop_op_gen,
        EVM_WORD_BYTES,
        WASM_I64_BYTES,
        WASM_I64_IN_EVM_WORD_COUNT,
    },
};
use core::{i64, u64};
#[cfg(test)]
use log::debug;

// recompute this value after adding or removing rwasm ops to jump()
pub const JUMP_PARAMS_COUNT: usize = 6;
pub fn jump<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    const OP: &str = "JUMP";
    const OPCODE: u8 = JUMP;
    #[cfg(test)]
    debug!("op:{}", OP);
    let is = translator.result_instruction_set_mut();
    gas!(translator, gas::constants::MID);
    const OP_PARAMS_COUNT: u64 = 1;

    let pc_from = translator.program_counter() - 1;
    let prev_opcode = unsafe { *translator.instruction_pointer_prev };
    if prev_opcode < PUSH0 || prev_opcode > PUSH32 {
        panic!("expected PUSHX opcode");
    }
    let push_count = compute_push_count(prev_opcode);
    let pc_prev = translator.program_counter_prev();
    let bytes_before = pc_from - pc_prev - 1;
    if bytes_before != push_count {
        panic!("expected distance {} got {}", push_count, bytes_before);
    };
    // const WASM_I64_BYTES_TMP: usize = 4;
    let mut pc_to_arr = [0u8; WASM_I64_BYTES];
    let mut bytes_to_fetch = if bytes_before < WASM_I64_BYTES {
        bytes_before
    } else {
        WASM_I64_BYTES
    };
    let pc_to_slice =
        translator.get_bytecode_slice(Some(-1 - bytes_to_fetch as isize), bytes_to_fetch);
    pc_to_arr[WASM_I64_BYTES - pc_to_slice.len()..].copy_from_slice(pc_to_slice);
    let pc_to = i64::from_be_bytes(pc_to_arr);
    translator.jumps_to_process_add(OPCODE, pc_from, pc_to as usize);
    let is = translator.result_instruction_set_mut();

    sp_drop_u256(is, OP_PARAMS_COUNT);

    let is_current_offset = is.len() as i64;
    // by default: just skips itself (will be replaced with real value later)
    is.op_i64_const(is_current_offset);
    is.op_br_indirect(2); // for i64_const and br_indirect itself
}

// recompute this value after adding or removing rwasm ops to jumpi()
pub const JUMPI_PARAMS_COUNT: usize = 31;
pub fn jumpi<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    const OP: &str = "JUMPI";
    const OPCODE: u8 = JUMPI;
    #[cfg(test)]
    debug!("op:{}", OP);
    let is = translator.result_instruction_set_mut();
    gas!(translator, gas::constants::HIGH);
    const OP_PARAMS_COUNT: u64 = 2;

    let pc_from = translator.program_counter() - 1;
    let prev_opcode = unsafe { *translator.instruction_pointer_prev };
    if prev_opcode < PUSH0 || prev_opcode > PUSH32 {
        panic!("expected PUSHX opcode");
    }
    let push_count = compute_push_count(prev_opcode);
    let pc_prev = translator.program_counter_prev();
    let bytes_before = pc_from - pc_prev - 1;
    if bytes_before != push_count {
        panic!("expected distance {} got {}", push_count, bytes_before);
    };
    // const WASM_I64_BYTES_TMP: usize = 4;
    let mut pc_to_arr = [0u8; WASM_I64_BYTES];
    let mut bytes_to_fetch = if bytes_before < WASM_I64_BYTES {
        bytes_before
    } else {
        WASM_I64_BYTES
    };
    let pc_to_slice =
        translator.get_bytecode_slice(Some(-1 - bytes_to_fetch as isize), bytes_to_fetch);
    pc_to_arr[WASM_I64_BYTES - pc_to_slice.len()..].copy_from_slice(pc_to_slice);
    let pc_to = u64::from_be_bytes(pc_to_arr);
    translator.jumps_to_process_add(OPCODE, pc_from, pc_to as usize);
    let is = translator.result_instruction_set_mut();

    sp_get_offset(is, None);
    sp_drop_u256(is, OP_PARAMS_COUNT);

    // fetch conditional param and make decision based on it
    is.op_i64_const(EVM_WORD_BYTES);
    is.op_i64_add();

    is.op_local_get(1);
    for i in 0..WASM_I64_IN_EVM_WORD_COUNT {
        if i > 0 {
            is.op_local_get(2);
            is.op_i64_const(i * WASM_I64_BYTES);
            is.op_i64_add();
        }
        load_i64_const(is, None);
        if i > 0 {
            is.op_i64_or();
        }
    }
    is.op_local_set(1);
    is.op_br_if_eqz(3);

    let current_offset = is.len() as i64;
    // by default: just skips itself (will be replaced with real value later)
    is.op_i64_const(current_offset);
    is.op_br_indirect(2); // for const and br_indirect itself
}

pub fn jumpdest<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    const OP: &str = "JUMPDEST";
    #[cfg(test)]
    debug!("op:{}", OP);
    let is = translator.result_instruction_set_mut();
    gas!(translator, gas::constants::JUMPDEST);
}

pub fn pc<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    const OP: &str = "PC";
    panic!("op:{} not implemented", OP);
    let is = translator.result_instruction_set_mut();
    gas!(translator, gas::constants::BASE);
}

pub fn ret<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    const OP: &str = "RET";
    #[cfg(test)]
    debug!("op:{}", OP);
    replace_with_call_to_subroutine(translator, host);
}

pub fn revert<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    const OP: &str = "REVERT";
    #[cfg(test)]
    debug!("op:{}", OP);
    replace_with_call_to_subroutine(translator, host);
}

pub fn stop<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    stop_op_gen(translator);
}

pub fn invalid<H: Host>(translator: &mut Translator<'_>, host: &mut H) {
    invalid_op_gen(translator);
}

pub fn not_found<H: Host>(translator: &mut Translator<'_>, _host: &mut H) {
    translator.instruction_result = InstructionResult::OpcodeNotFound;
}
