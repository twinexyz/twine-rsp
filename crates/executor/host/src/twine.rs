use reth_evm::eth::EthEvmContext;
use reth_evm::{Database, EthEvm, EvmEnv, EvmFactory};
use revm::context::result::{EVMError, HaltReason};
use revm::context::{Cfg, ContextTr, TxEnv};
use revm::handler::{EthPrecompiles, PrecompileProvider};
use revm::inspector::NoOpInspector;
use revm::interpreter::interpreter::EthInterpreter;
use revm::interpreter::{InputsImpl, InterpreterResult};
use revm::precompile::Precompiles;
use revm::{Context, Inspector, MainBuilder, MainContext};
use revm_primitives::hardfork::SpecId;
use revm_primitives::Address;
use twine_constants::precompiles::{
    TWINE_CONSENSUS_VERIFIER_PRECOMPILE_ADDRESS, TWINE_TRANSACTION_PRECOMPILE_ADDRESS,
};
use twine_l1_consensus_verifier_precompile::ConsensusVerifierPrecompile;
use twine_l1_transactions_precompile::TransactionPrecompile;

/// Precompile for twine
#[derive(Clone, Debug)]
pub struct TwineCustomPrecompile {
    pub inner: EthPrecompiles,
    pub l1_consensus: Address,
    pub l1_transaction: Address,
}

impl Default for TwineCustomPrecompile {
    fn default() -> Self {
        Self {
            inner: EthPrecompiles::default(),
            l1_consensus: TWINE_CONSENSUS_VERIFIER_PRECOMPILE_ADDRESS,
            l1_transaction: TWINE_TRANSACTION_PRECOMPILE_ADDRESS,
        }
    }
}

impl<CTX: ContextTr> PrecompileProvider<CTX> for TwineCustomPrecompile {
    type Output = InterpreterResult;

    fn set_spec(&mut self, spec: <CTX::Cfg as Cfg>::Spec) -> bool {
        self.inner = EthPrecompiles { precompiles: Precompiles::prague(), spec: spec.into() };
        self.l1_consensus = TWINE_CONSENSUS_VERIFIER_PRECOMPILE_ADDRESS;
        self.l1_transaction = TWINE_TRANSACTION_PRECOMPILE_ADDRESS;
        true
    }

    fn run(
        &mut self,
        context: &mut CTX,
        address: &Address,
        inputs: &InputsImpl,
        is_static: bool,
        gas_limit: u64,
    ) -> Result<Option<Self::Output>, String> {
        {
            if address.eq(&self.l1_transaction) {
                return TransactionPrecompile::run(context, address, inputs, is_static, gas_limit);
            }
        }

        {
            if address.eq(&self.l1_consensus) {
                return ConsensusVerifierPrecompile::run(
                    context, address, inputs, is_static, gas_limit,
                );
            }
        }

        self.inner.run(context, address, inputs, is_static, gas_limit)
    }

    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        let default_addresses = self.inner.warm_addresses();
        Box::new(default_addresses.chain(vec![
            TWINE_CONSENSUS_VERIFIER_PRECOMPILE_ADDRESS,
            TWINE_TRANSACTION_PRECOMPILE_ADDRESS,
        ]))
    }

    fn contains(&self, address: &Address) -> bool {
        self.inner.contains(address)
            || TWINE_CONSENSUS_VERIFIER_PRECOMPILE_ADDRESS.eq(address)
            || TWINE_TRANSACTION_PRECOMPILE_ADDRESS.eq(address)
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct TwineEvmFactory;

impl TwineEvmFactory {
    pub fn new() -> Self {
        Self
    }
}

impl EvmFactory for TwineEvmFactory {
    type Context<DB: Database> = EthEvmContext<DB>;
    type Error<DBError: core::error::Error + Send + Sync + 'static> = EVMError<DBError>;
    type Evm<DB: Database, I: Inspector<EthEvmContext<DB>, EthInterpreter>> =
        EthEvm<DB, I, TwineCustomPrecompile>;
    type HaltReason = HaltReason;
    type Spec = SpecId;
    type Tx = TxEnv;

    fn create_evm<DB: Database>(&self, db: DB, input: EvmEnv) -> Self::Evm<DB, NoOpInspector> {
        let evm = Context::mainnet()
            .with_db(db)
            .with_cfg(input.cfg_env)
            .with_block(input.block_env)
            .build_mainnet_with_inspector(NoOpInspector {})
            .with_precompiles(TwineCustomPrecompile::default());

        EthEvm::new(evm, false)
    }

    fn create_evm_with_inspector<DB: Database, I: Inspector<Self::Context<DB>, EthInterpreter>>(
        &self,
        db: DB,
        input: EvmEnv,
        inspector: I,
    ) -> Self::Evm<DB, I> {
        EthEvm::new(self.create_evm(db, input).into_inner().with_inspector(inspector), true)
    }
}
