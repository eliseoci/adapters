pub mod contract;
pub mod error;
pub mod msg;
mod staking;

pub const TENDERMINT_STAKING: &str = "abstract:tendermint-staking";

#[cfg(feature = "interface")]
pub mod cw_orch {

    use crate::msg::*;
    use abstract_interface::AdapterDeployer;
    use cosmwasm_std::Empty;
    use cw_orch::environment::CwEnv;
    use cw_orch::interface;
    use cw_orch::prelude::ContractWrapper;
    use cw_orch::prelude::*;

    #[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
    pub struct TMintStakingAdapter;

    impl<Chain: CwEnv> AdapterDeployer<Chain, Empty> for TMintStakingAdapter<Chain> {}

    impl<Chain: CwEnv> Uploadable for TMintStakingAdapter<Chain> {
        fn wrapper(&self) -> <Mock as TxHandler>::ContractSource {
            Box::new(ContractWrapper::new_with_empty(
                crate::contract::execute,
                crate::contract::instantiate,
                crate::contract::query,
            ))
        }
        fn wasm(&self) -> WasmPath {
            artifacts_dir_from_workspace!()
                .find_wasm_path("abstract_tendermint_staking_adapter")
                .unwrap()
        }
    }
}
