use super::Opcode;
use crate::{
    circuit_input_builder::{CircuitInputStateRef, ExecStep},
    operation::CallContextField,
    Error,
};
use eth_types::{GethExecStep, ToWord};

/// Placeholder structure used to implement [`Opcode`] trait over it
/// corresponding to the [`OpcodeId::CALLER`](crate::evm::OpcodeId::CALLER) `OpcodeId`.
#[derive(Debug, Copy, Clone)]
pub(crate) struct Caller;

impl Opcode for Caller {
    fn gen_associated_ops(
        state: &mut CircuitInputStateRef,
        geth_steps: &[GethExecStep],
    ) -> Result<Vec<ExecStep>, Error> {
        let geth_step = &geth_steps[0];
        let mut exec_step = state.new_step(geth_step)?;
        // Get caller_address result from next step
        let caller_address = state.call()?.caller_address.to_word();
        // CallContext read of the caller_address
        state.call_context_read(
            &mut exec_step,
            state.call()?.call_id,
            CallContextField::CallerAddress,
            caller_address,
        )?;

        // Stack write of the caller_address
        #[cfg(feature = "enable-stack")]
        assert_eq!(caller_address, geth_steps[1].stack.last()?);
        state.stack_push(&mut exec_step, caller_address)?;

        Ok(vec![exec_step])
    }
}

#[cfg(test)]
mod caller_tests {
    use super::*;
    use crate::{
        circuit_input_builder::ExecState,
        mock::BlockData,
        operation::{CallContextOp, StackOp, RW},
    };
    use eth_types::{
        bytecode,
        evm_types::{OpcodeId, StackAddress},
        geth_types::GethData,
        ToWord,
    };

    use mock::test_ctx::{helpers::*, TestContext};
    use pretty_assertions::assert_eq;

    #[test]
    fn caller_opcode_impl() {
        let code = bytecode! {
            CALLER
            STOP
        };

        // Get the execution steps from the external tracer
        let block: GethData = TestContext::<2, 1>::new(
            None,
            account_0_code_account_1_no_code(code),
            tx_from_1_to_0,
            |block, _tx| block.number(0xcafeu64),
        )
        .unwrap()
        .into();

        let mut builder = BlockData::new_from_geth_data(block.clone()).new_circuit_input_builder();
        builder
            .handle_block(&block.eth_block, &block.geth_traces)
            .unwrap();

        let step = builder.block.txs()[0]
            .steps()
            .iter()
            .find(|step| step.exec_state == ExecState::Op(OpcodeId::CALLER))
            .unwrap();

        let call_id = builder.block.txs()[0].calls()[0].call_id;
        let caller_address = block.eth_block.transactions[0].from.to_word();
        assert_eq!(
            {
                let operation =
                    &builder.block.container.call_context[step.bus_mapping_instance[0].as_usize()];
                (operation.rw(), operation.op())
            },
            (
                RW::READ,
                &CallContextOp {
                    call_id,
                    field: CallContextField::CallerAddress,
                    value: caller_address,
                }
            )
        );
        assert_eq!(
            {
                let operation =
                    &builder.block.container.stack[step.bus_mapping_instance[1].as_usize()];
                (operation.rw(), operation.op())
            },
            (
                RW::WRITE,
                &StackOp::new(1, StackAddress::from(1023), caller_address)
            )
        );
    }
}
