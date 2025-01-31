use crate::msg::StakingQueryMsg;
use crate::{
    contract::{CwStakingAdapter, StakingResult},
    providers::resolver::{self, is_over_ibc},
};
use abstract_sdk::features::AbstractNameService;
use cosmwasm_std::{to_binary, Binary, Deps, Env, StdError};

/// Handle queries related to staking
pub fn query_handler(
    deps: Deps,
    env: Env,
    app: &CwStakingAdapter,
    msg: StakingQueryMsg,
) -> StakingResult<Binary> {
    let name_service = app.name_service(deps);
    let ans_host = name_service.host();

    match msg {
        StakingQueryMsg::Info {
            provider,
            staking_token,
        } => {
            // if provider is on an app-chain, error
            if is_over_ibc(&provider)? {
                Err(crate::error::StakingError::IbcQueryNotSupported)
            } else {
                // the query can be executed on the local chain
                let mut provider = resolver::resolve_local_provider(&provider)
                    .map_err(|e| StdError::generic_err(e.to_string()))?;
                provider.fetch_data(deps, env, ans_host, staking_token)?;
                Ok(to_binary(&provider.query_info(&deps.querier)?)?)
            }
        }
        StakingQueryMsg::Staked {
            provider,
            staking_token,
            staker_address,
            unbonding_period,
        } => {
            // if provider is on an app-chain, error
            if is_over_ibc(&provider)? {
                Err(crate::error::StakingError::IbcQueryNotSupported)
            } else {
                // the query can be executed on the local chain
                let mut provider = resolver::resolve_local_provider(&provider)
                    .map_err(|e| StdError::generic_err(e.to_string()))?;
                provider.fetch_data(deps, env, ans_host, staking_token)?;
                Ok(to_binary(&provider.query_staked(
                    &deps.querier,
                    deps.api.addr_validate(&staker_address)?,
                    unbonding_period,
                )?)?)
            }
        }
        StakingQueryMsg::Unbonding {
            provider,
            staking_token,
            staker_address,
        } => {
            // if provider is on an app-chain, error
            if is_over_ibc(&provider)? {
                Err(crate::error::StakingError::IbcQueryNotSupported)
            } else {
                // the query can be executed on the local chain
                let mut provider = resolver::resolve_local_provider(&provider)
                    .map_err(|e| StdError::generic_err(e.to_string()))?;
                provider.fetch_data(deps, env, ans_host, staking_token)?;
                Ok(to_binary(&provider.query_unbonding(
                    &deps.querier,
                    deps.api.addr_validate(&staker_address)?,
                )?)?)
            }
        }
        StakingQueryMsg::RewardTokens {
            provider,
            staking_token,
        } => {
            // if provider is on an app-chain, error
            if is_over_ibc(&provider)? {
                Err(crate::error::StakingError::IbcQueryNotSupported)
            } else {
                // the query can be executed on the local chain
                let mut provider = resolver::resolve_local_provider(&provider)
                    .map_err(|e| StdError::generic_err(e.to_string()))?;
                provider.fetch_data(deps, env, ans_host, staking_token)?;
                Ok(to_binary(&provider.query_rewards(&deps.querier)?)?)
            }
        }
    }
}
