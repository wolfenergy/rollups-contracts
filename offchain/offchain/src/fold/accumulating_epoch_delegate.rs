use offchain_core::ethers;

use super::input_contract_address_delegate::InputContractAddressFoldDelegate;
use super::input_delegate::InputFoldDelegate;
use super::types::{AccumulatingEpoch, EpochInputState};

use offchain_core::types::Block;
use state_fold::{
    delegate_access::{FoldAccess, SyncAccess},
    error::*,
    types::*,
    DelegateAccess, StateFold,
};

use async_trait::async_trait;
use std::sync::Arc;

use ethers::types::{Address, H256, U256};

/// Accumulating epoch StateFold Delegate
pub struct AccumulatingEpochFoldDelegate<DA: DelegateAccess> {
    input_fold: Arc<StateFold<InputFoldDelegate, DA>>,
    input_contract_address_fold:
        Arc<StateFold<InputContractAddressFoldDelegate, DA>>,
}

impl<DA: DelegateAccess> AccumulatingEpochFoldDelegate<DA> {
    pub fn new(
        input_fold: Arc<StateFold<InputFoldDelegate, DA>>,
        input_contract_address_fold: Arc<
            StateFold<InputContractAddressFoldDelegate, DA>,
        >,
    ) -> Self {
        Self {
            input_fold,
            input_contract_address_fold,
        }
    }
}

#[async_trait]
impl<DA: DelegateAccess + Send + Sync + 'static> StateFoldDelegate
    for AccumulatingEpochFoldDelegate<DA>
{
    type InitialState = (Address, U256);
    type Accumulator = AccumulatingEpoch;
    type State = BlockState<Self::Accumulator>;

    async fn sync<A: SyncAccess + Send + Sync>(
        &self,
        initial_state: &(Address, U256),
        block: &Block,
        _access: &A,
    ) -> SyncResult<Self::Accumulator, A> {
        let (descartesv2_contract_address, epoch_number) = *initial_state;

        let input_contract_address = self
            .get_input_contract_address_sync(
                descartesv2_contract_address,
                block.hash,
            )
            .await?;

        // Inputs of epoch
        let inputs = self
            .get_inputs_sync(input_contract_address, epoch_number, block.hash)
            .await?;

        Ok(AccumulatingEpoch {
            inputs,
            epoch_number,
            descartesv2_contract_address,
            input_contract_address,
        })
    }

    async fn fold<A: FoldAccess + Send + Sync>(
        &self,
        previous_state: &Self::Accumulator,
        block: &Block,
        _access: &A,
    ) -> FoldResult<Self::Accumulator, A> {
        let epoch_number = previous_state.epoch_number.clone();
        let descartesv2_contract_address =
            previous_state.descartesv2_contract_address.clone();
        let input_contract_address =
            previous_state.input_contract_address.clone();

        // Inputs of epoch
        let inputs = self
            .get_inputs_fold(input_contract_address, epoch_number, block.hash)
            .await?;

        Ok(AccumulatingEpoch {
            epoch_number,
            inputs,
            descartesv2_contract_address,
            input_contract_address,
        })
    }

    fn convert(
        &self,
        accumulator: &BlockState<Self::Accumulator>,
    ) -> Self::State {
        accumulator.clone()
    }
}

impl<DA: DelegateAccess + Send + Sync + 'static>
    AccumulatingEpochFoldDelegate<DA>
{
    async fn get_input_contract_address_sync<
        A: SyncAccess + Send + Sync + 'static,
    >(
        &self,
        descartesv2_contract_address: Address,
        block_hash: H256,
    ) -> SyncResult<Address, A> {
        Ok(self
            .input_contract_address_fold
            .get_state_for_block(
                &descartesv2_contract_address,
                Some(block_hash),
            )
            .await
            .map_err(|e| {
                SyncDelegateError {
                    err: format!(
                        "input contract address state fold error: {:?}",
                        e
                    ),
                }
                .build()
            })?
            .state
            .input_contract_address)
    }

    async fn get_inputs_sync<A: SyncAccess + Send + Sync + 'static>(
        &self,
        input_contract_address: Address,
        epoch: U256,
        block_hash: H256,
    ) -> SyncResult<EpochInputState, A> {
        Ok(self
            .input_fold
            .get_state_for_block(
                &(input_contract_address, epoch),
                Some(block_hash),
            )
            .await
            .map_err(|e| {
                SyncDelegateError {
                    err: format!("Input state fold error: {:?}", e),
                }
                .build()
            })?
            .state)
    }

    async fn get_inputs_fold<A: FoldAccess + Send + Sync + 'static>(
        &self,
        input_contract_address: Address,
        epoch: U256,
        block_hash: H256,
    ) -> FoldResult<EpochInputState, A> {
        Ok(self
            .input_fold
            .get_state_for_block(
                &(input_contract_address, epoch),
                Some(block_hash),
            )
            .await
            .map_err(|e| {
                FoldDelegateError {
                    err: format!("Input state fold error: {:?}", e),
                }
                .build()
            })?
            .state)
    }
}
