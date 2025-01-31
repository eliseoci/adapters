use crate::error::DexError;
use crate::exchanges::exchange_resolver;
use crate::msg::{DexAction, DexExecuteMsg, DexName, IBC_DEX_ID};
use crate::{
    contract::{DexAdapter, DexResult},
    state::SWAP_FEE,
};
use abstract_core::ibc_client::CallbackInfo;
use abstract_core::objects::ans_host::AnsHost;
use abstract_core::objects::AnsAsset;
use abstract_sdk::{features::AbstractNameService, Execution};
use abstract_sdk::{AccountVerification, IbcInterface, Resolve};
use cosmwasm_std::{to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError};

const ACTION_RETRIES: u8 = 3;

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: DexAdapter,
    msg: DexExecuteMsg,
) -> DexResult {
    match msg {
        DexExecuteMsg::Action {
            dex: dex_name,
            action,
        } => {
            let exchange = exchange_resolver::identify_exchange(&dex_name)?;
            // if exchange is on an app-chain, execute the action on the app-chain
            if exchange.over_ibc() {
                handle_ibc_request(&deps, info, &adapter, dex_name, &action)
            } else {
                // the action can be executed on the local chain
                handle_local_request(deps, env, info, adapter, action, dex_name)
            }
        }
        DexExecuteMsg::UpdateFee {
            swap_fee,
            recipient_account: recipient_account_id,
        } => {
            // only previous OS can change the owner
            adapter
                .account_registry(deps.as_ref())
                .assert_proxy(&info.sender)?;
            if let Some(swap_fee) = swap_fee {
                let mut fee = SWAP_FEE.load(deps.storage)?;
                fee.set_share(swap_fee)?;
                SWAP_FEE.save(deps.storage, &fee)?;
            }

            if let Some(account_id) = recipient_account_id {
                let mut fee = SWAP_FEE.load(deps.storage)?;
                let recipient = adapter
                    .account_registry(deps.as_ref())
                    .proxy_address(account_id)?;
                fee.set_recipient(deps.api, recipient)?;
                SWAP_FEE.save(deps.storage, &fee)?;
            }
            Ok(Response::default())
        }
    }
}

/// Handle an adapter request that can be executed on the local chain
fn handle_local_request(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    adapter: DexAdapter,
    action: DexAction,
    exchange: String,
) -> DexResult {
    let exchange = exchange_resolver::resolve_exchange(&exchange)?;
    let (msgs, _) = crate::traits::adapter::DexAdapter::resolve_dex_action(
        &adapter,
        deps.as_ref(),
        action,
        exchange,
    )?;
    let proxy_msg = adapter
        .executor(deps.as_ref())
        .execute(msgs.into_iter().map(Into::into).collect())?;
    Ok(Response::new().add_message(proxy_msg))
}

/// Handle an adapter request that can be executed on an IBC chain
fn handle_ibc_request(
    deps: &DepsMut,
    info: MessageInfo,
    adapter: &DexAdapter,
    dex_name: DexName,
    action: &DexAction,
) -> DexResult {
    let host_chain = dex_name;
    let ans = adapter.name_service(deps.as_ref());
    let ibc_client = adapter.ibc_client(deps.as_ref());
    // get the to-be-sent assets from the action
    let coins = resolve_assets_to_transfer(deps.as_ref(), action, ans.host())?;
    // construct the ics20 call(s)
    let ics20_transfer_msg = ibc_client.ics20_transfer(host_chain.clone(), coins)?;
    // construct the action to be called on the host
    let action = abstract_sdk::core::ibc_host::HostAction::App {
        msg: to_binary(&action)?,
    };
    let maybe_contract_info = deps.querier.query_wasm_contract_info(info.sender.clone());
    let callback = if maybe_contract_info.is_err() {
        None
    } else {
        Some(CallbackInfo {
            id: IBC_DEX_ID.to_string(),
            receiver: info.sender.into_string(),
        })
    };
    let ibc_action_msg = ibc_client.host_action(host_chain, action, callback, ACTION_RETRIES)?;

    // call both messages on the proxy
    Ok(Response::new().add_messages(vec![ics20_transfer_msg, ibc_action_msg]))
}

pub(crate) fn resolve_assets_to_transfer(
    deps: Deps,
    dex_action: &DexAction,
    ans_host: &AnsHost,
) -> DexResult<Vec<Coin>> {
    // resolve asset to native asset
    let offer_to_coin = |offer: &AnsAsset| {
        offer
            .resolve(&deps.querier, ans_host)?
            .try_into()
            .map_err(DexError::from)
    };

    match dex_action {
        DexAction::ProvideLiquidity { assets, .. } => {
            let coins: Result<Vec<Coin>, _> = assets.iter().map(offer_to_coin).collect();
            coins
        }
        DexAction::ProvideLiquiditySymmetric { .. } => Err(DexError::Std(StdError::generic_err(
            "Cross-chain symmetric provide liquidity not supported.",
        ))),
        DexAction::WithdrawLiquidity { lp_token, amount } => Ok(vec![offer_to_coin(&AnsAsset {
            name: lp_token.to_owned(),
            amount: amount.to_owned(),
        })?]),
        DexAction::Swap { offer_asset, .. } => Ok(vec![offer_to_coin(offer_asset)?]),
        DexAction::CustomSwap { offer_assets, .. } => {
            let coins: Result<Vec<Coin>, _> = offer_assets.iter().map(offer_to_coin).collect();
            coins
        }
    }
    .map_err(Into::into)
}
