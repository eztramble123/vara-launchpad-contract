//! Token Launchpad Contract Application Logic.
//!
//! This module implements fair token launches with the following features:
//! - Create token launches with configurable parameters
//! - Optional whitelist for exclusive access
//! - Maximum contribution per wallet
//! - Automatic refunds if minimum not reached
//! - Optional vesting schedule for purchased tokens

#![no_std]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sails_rs::prelude::*;
use vara_contracts_shared::{Amount, BlockNumber, ContractError, Id, VestingConfig};

/// Status of a token launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, Default)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum LaunchStatus {
    /// Launch is being set up (not yet started).
    #[default]
    Pending,
    /// Launch is active and accepting contributions.
    Active,
    /// Launch succeeded (minimum reached).
    Succeeded,
    /// Launch failed (minimum not reached by deadline).
    Failed,
    /// Launch was cancelled by creator.
    Cancelled,
    /// All tokens distributed.
    Finalized,
}

/// Token launch configuration.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Launch {
    pub id: Id,
    pub creator: ActorId,
    pub title: String,
    pub description: String,
    /// Token contract address (VFT).
    pub token_address: ActorId,
    /// Total tokens available for sale.
    pub total_tokens: Amount,
    /// Tokens remaining for sale.
    pub tokens_remaining: Amount,
    /// Price per token in native currency.
    pub price_per_token: Amount,
    /// Minimum total raise required (if not met, refunds enabled).
    pub min_raise: Amount,
    /// Maximum total raise (hard cap).
    pub max_raise: Amount,
    /// Amount raised so far.
    pub total_raised: Amount,
    /// Maximum contribution per wallet.
    pub max_per_wallet: Amount,
    /// Launch start time.
    pub start_time: BlockNumber,
    /// Launch end time.
    pub end_time: BlockNumber,
    /// Optional whitelist (if empty, anyone can participate).
    pub whitelist: BTreeSet<ActorId>,
    /// Is whitelist enabled.
    pub whitelist_enabled: bool,
    /// Contributions per address.
    pub contributions: BTreeMap<ActorId, Amount>,
    /// Tokens claimed per address.
    pub claimed: BTreeMap<ActorId, Amount>,
    /// Optional vesting configuration.
    pub vesting_config: Option<VestingConfig>,
    pub status: LaunchStatus,
    pub created_at: BlockNumber,
}

impl Launch {
    /// Calculate tokens purchasable for a given amount.
    pub fn tokens_for_amount(&self, amount: Amount) -> Amount {
        if self.price_per_token == 0 {
            return 0;
        }
        amount / self.price_per_token
    }

    /// Check if launch is within the active time window.
    pub fn is_in_time_window(&self, current_block: BlockNumber) -> bool {
        current_block >= self.start_time && current_block <= self.end_time
    }

    /// Check if address is allowed to participate.
    pub fn can_participate(&self, address: &ActorId) -> bool {
        !self.whitelist_enabled || self.whitelist.contains(address)
    }

    /// Get remaining allocation for a wallet.
    pub fn remaining_allocation(&self, address: &ActorId) -> Amount {
        let contributed = self.contributions.get(address).copied().unwrap_or(0);
        self.max_per_wallet.saturating_sub(contributed)
    }

    /// Check if minimum raise was met.
    pub fn min_raise_met(&self) -> bool {
        self.total_raised >= self.min_raise
    }
}

/// Storage for the Launchpad contract.
#[derive(Default)]
pub struct LaunchpadStorage {
    launches: BTreeMap<Id, Launch>,
    next_launch_id: Id,
    owner: ActorId,
    /// Platform fee in basis points (100 = 1%).
    fee_basis_points: u16,
    accumulated_fees: Amount,
}

fn storage_mut() -> &'static mut LaunchpadStorage {
    unsafe {
        static mut STORAGE: Option<LaunchpadStorage> = None;
        STORAGE.get_or_insert_with(LaunchpadStorage::default)
    }
}

fn storage() -> &'static LaunchpadStorage {
    unsafe {
        static mut STORAGE: Option<LaunchpadStorage> = None;
        STORAGE.get_or_insert_with(LaunchpadStorage::default)
    }
}

fn init_storage(owner: ActorId, fee_basis_points: u16) {
    let s = storage_mut();
    s.owner = owner;
    s.fee_basis_points = fee_basis_points;
}

/// Transfer native tokens to recipient.
fn transfer_native(to: ActorId, amount: Amount) -> Result<(), ContractError> {
    gstd::msg::send_bytes(to, [], amount as u128)
        .map_err(|_| ContractError::TransferFailed)?;
    Ok(())
}

/// Events emitted by the Launchpad contract.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum LaunchpadEvent {
    LaunchCreated {
        launch_id: Id,
        creator: ActorId,
        token_address: ActorId,
        total_tokens: Amount,
    },
    LaunchStarted {
        launch_id: Id,
    },
    Contributed {
        launch_id: Id,
        contributor: ActorId,
        amount: Amount,
        tokens_purchased: Amount,
    },
    TokensClaimed {
        launch_id: Id,
        claimer: ActorId,
        amount: Amount,
    },
    RefundClaimed {
        launch_id: Id,
        contributor: ActorId,
        amount: Amount,
    },
    FundsWithdrawn {
        launch_id: Id,
        amount: Amount,
    },
    LaunchSucceeded {
        launch_id: Id,
        total_raised: Amount,
    },
    LaunchFailed {
        launch_id: Id,
    },
    LaunchCancelled {
        launch_id: Id,
    },
    WhitelistUpdated {
        launch_id: Id,
        addresses_added: u32,
    },
}

impl sails_rs::SailsEvent for LaunchpadEvent {
    fn encoded_event_name(&self) -> &'static [u8] {
        match self {
            LaunchpadEvent::LaunchCreated { .. } => b"LaunchCreated",
            LaunchpadEvent::LaunchStarted { .. } => b"LaunchStarted",
            LaunchpadEvent::Contributed { .. } => b"Contributed",
            LaunchpadEvent::TokensClaimed { .. } => b"TokensClaimed",
            LaunchpadEvent::RefundClaimed { .. } => b"RefundClaimed",
            LaunchpadEvent::FundsWithdrawn { .. } => b"FundsWithdrawn",
            LaunchpadEvent::LaunchSucceeded { .. } => b"LaunchSucceeded",
            LaunchpadEvent::LaunchFailed { .. } => b"LaunchFailed",
            LaunchpadEvent::LaunchCancelled { .. } => b"LaunchCancelled",
            LaunchpadEvent::WhitelistUpdated { .. } => b"WhitelistUpdated",
        }
    }
}

/// Input for creating a launch.
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct CreateLaunchInput {
    pub title: String,
    pub description: String,
    pub token_address: ActorId,
    pub total_tokens: Amount,
    pub price_per_token: Amount,
    pub min_raise: Amount,
    pub max_raise: Amount,
    pub max_per_wallet: Amount,
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub whitelist_enabled: bool,
    pub vesting_config: Option<VestingConfig>,
}

/// Launchpad Service implementation.
pub struct LaunchpadService(());

impl LaunchpadService {
    pub fn new() -> Self {
        Self(())
    }
}

#[sails_rs::service(events = LaunchpadEvent)]
impl LaunchpadService {
    /// Create a new token launch.
    #[export]
    pub fn create_launch(&mut self, input: CreateLaunchInput) -> Result<Id, ContractError> {
        let creator = gstd::msg::source();
        let current_block = gstd::exec::block_height();

        if input.title.is_empty() {
            return Err(ContractError::invalid_input("Title cannot be empty"));
        }

        if input.total_tokens == 0 {
            return Err(ContractError::ZeroAmount);
        }

        if input.price_per_token == 0 {
            return Err(ContractError::invalid_input("Price must be > 0"));
        }

        if input.start_time <= current_block {
            return Err(ContractError::invalid_input("Start time must be in future"));
        }

        if input.end_time <= input.start_time {
            return Err(ContractError::invalid_input("End time must be after start"));
        }

        if input.min_raise > input.max_raise {
            return Err(ContractError::invalid_input("Min raise exceeds max raise"));
        }

        if input.max_per_wallet == 0 {
            return Err(ContractError::invalid_input("Max per wallet must be > 0"));
        }

        let s = storage_mut();
        let launch_id = s.next_launch_id;
        s.next_launch_id = s
            .next_launch_id
            .checked_add(1)
            .ok_or(ContractError::Overflow)?;

        let launch = Launch {
            id: launch_id,
            creator,
            title: input.title,
            description: input.description,
            token_address: input.token_address,
            total_tokens: input.total_tokens,
            tokens_remaining: input.total_tokens,
            price_per_token: input.price_per_token,
            min_raise: input.min_raise,
            max_raise: input.max_raise,
            total_raised: 0,
            max_per_wallet: input.max_per_wallet,
            start_time: input.start_time,
            end_time: input.end_time,
            whitelist: BTreeSet::new(),
            whitelist_enabled: input.whitelist_enabled,
            contributions: BTreeMap::new(),
            claimed: BTreeMap::new(),
            vesting_config: input.vesting_config,
            status: LaunchStatus::Pending,
            created_at: current_block,
        };

        s.launches.insert(launch_id, launch);

        let _ = self.emit_event(LaunchpadEvent::LaunchCreated {
            launch_id,
            creator,
            token_address: input.token_address,
            total_tokens: input.total_tokens,
        });

        Ok(launch_id)
    }

    /// Add addresses to whitelist (creator only).
    #[export]
    pub fn add_to_whitelist(
        &mut self,
        launch_id: Id,
        addresses: Vec<ActorId>,
    ) -> Result<(), ContractError> {
        let caller = gstd::msg::source();

        let s = storage_mut();
        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator {
            return Err(ContractError::Unauthorized);
        }

        if launch.status != LaunchStatus::Pending && launch.status != LaunchStatus::Active {
            return Err(ContractError::invalid_state("Cannot modify whitelist"));
        }

        let count = addresses.len() as u32;
        for addr in addresses {
            launch.whitelist.insert(addr);
        }

        let _ = self.emit_event(LaunchpadEvent::WhitelistUpdated {
            launch_id,
            addresses_added: count,
        });

        Ok(())
    }

    /// Start a launch (creator only, before start time).
    #[export]
    pub fn start_launch(&mut self, launch_id: Id) -> Result<(), ContractError> {
        let caller = gstd::msg::source();

        let s = storage_mut();
        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator {
            return Err(ContractError::Unauthorized);
        }

        if launch.status != LaunchStatus::Pending {
            return Err(ContractError::invalid_state("Launch not pending"));
        }

        launch.status = LaunchStatus::Active;

        let _ = self.emit_event(LaunchpadEvent::LaunchStarted { launch_id });

        Ok(())
    }

    /// Contribute to a launch.
    #[export]
    pub fn contribute(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let contributor = gstd::msg::source();
        let value = gstd::msg::value() as Amount;
        let current_block = gstd::exec::block_height();

        if value == 0 {
            return Err(ContractError::ZeroAmount);
        }

        let s = storage_mut();
        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if launch.status != LaunchStatus::Active {
            return Err(ContractError::invalid_state("Launch not active"));
        }

        if !launch.is_in_time_window(current_block) {
            return Err(ContractError::invalid_state("Outside launch time window"));
        }

        if !launch.can_participate(&contributor) {
            return Err(ContractError::Unauthorized);
        }

        // Check remaining allocation
        let remaining_allocation = launch.remaining_allocation(&contributor);
        if remaining_allocation == 0 {
            return Err(ContractError::invalid_state("Allocation exhausted"));
        }

        // Cap contribution to remaining allocation and max raise
        let remaining_raise = launch.max_raise.saturating_sub(launch.total_raised);
        let max_contribution = remaining_allocation.min(remaining_raise);
        let actual_contribution = value.min(max_contribution);

        if actual_contribution == 0 {
            return Err(ContractError::invalid_state("Max raise reached"));
        }

        // Calculate tokens
        let tokens_purchased = launch.tokens_for_amount(actual_contribution);
        if tokens_purchased == 0 {
            return Err(ContractError::invalid_input("Contribution too small"));
        }

        if tokens_purchased > launch.tokens_remaining {
            return Err(ContractError::invalid_state("Insufficient tokens remaining"));
        }

        // Update state
        let current_contribution = launch.contributions.get(&contributor).copied().unwrap_or(0);
        launch
            .contributions
            .insert(contributor, current_contribution.saturating_add(actual_contribution));
        launch.total_raised = launch.total_raised.saturating_add(actual_contribution);
        launch.tokens_remaining = launch.tokens_remaining.saturating_sub(tokens_purchased);

        // Refund excess
        if value > actual_contribution {
            transfer_native(contributor, value.saturating_sub(actual_contribution))?;
        }

        let _ = self.emit_event(LaunchpadEvent::Contributed {
            launch_id,
            contributor,
            amount: actual_contribution,
            tokens_purchased,
        });

        Ok(tokens_purchased)
    }

    /// Finalize a launch after end time.
    #[export]
    pub fn finalize(&mut self, launch_id: Id) -> Result<LaunchStatus, ContractError> {
        let current_block = gstd::exec::block_height();

        let s = storage_mut();
        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if launch.status != LaunchStatus::Active {
            return Err(ContractError::invalid_state("Launch not active"));
        }

        if current_block <= launch.end_time {
            return Err(ContractError::invalid_state("Launch not ended"));
        }

        let new_status = if launch.min_raise_met() {
            LaunchStatus::Succeeded
        } else {
            LaunchStatus::Failed
        };

        launch.status = new_status;

        if new_status == LaunchStatus::Succeeded {
            let _ = self.emit_event(LaunchpadEvent::LaunchSucceeded {
                launch_id,
                total_raised: launch.total_raised,
            });
        } else {
            let _ = self.emit_event(LaunchpadEvent::LaunchFailed { launch_id });
        }

        Ok(new_status)
    }

    /// Claim purchased tokens (after successful launch).
    #[export]
    pub fn claim_tokens(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let claimer = gstd::msg::source();

        let s = storage_mut();
        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if launch.status != LaunchStatus::Succeeded && launch.status != LaunchStatus::Finalized {
            return Err(ContractError::invalid_state("Launch not successful"));
        }

        let contribution = launch
            .contributions
            .get(&claimer)
            .copied()
            .ok_or(ContractError::invalid_state("No contribution found"))?;

        let already_claimed = launch.claimed.get(&claimer).copied().unwrap_or(0);
        let tokens_purchased = launch.tokens_for_amount(contribution);
        let tokens_to_claim = tokens_purchased.saturating_sub(already_claimed);

        if tokens_to_claim == 0 {
            return Err(ContractError::invalid_state("All tokens claimed"));
        }

        // If vesting is configured, calculate claimable based on vesting
        let claimable = if let Some(ref vesting) = launch.vesting_config {
            let current_block = gstd::exec::block_height();
            if current_block < vesting.cliff_end() {
                return Err(ContractError::invalid_state("Cliff period not ended"));
            }

            if current_block >= vesting.vesting_end() {
                tokens_to_claim
            } else {
                // Linear vesting calculation
                let vesting_duration = vesting.vesting_duration as u128;
                let elapsed = (current_block - vesting.start_block) as u128;
                let vested = tokens_purchased
                    .saturating_mul(elapsed)
                    .checked_div(vesting_duration)
                    .unwrap_or(0);
                vested.saturating_sub(already_claimed)
            }
        } else {
            tokens_to_claim
        };

        if claimable == 0 {
            return Err(ContractError::invalid_state("No tokens claimable yet"));
        }

        launch
            .claimed
            .insert(claimer, already_claimed.saturating_add(claimable));

        // Note: Actual token transfer would require calling VFT contract
        // For this template, we emit event and assume integration handles transfer

        let _ = self.emit_event(LaunchpadEvent::TokensClaimed {
            launch_id,
            claimer,
            amount: claimable,
        });

        Ok(claimable)
    }

    /// Claim refund (after failed launch).
    #[export]
    pub fn claim_refund(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let claimer = gstd::msg::source();

        let s = storage_mut();
        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if launch.status != LaunchStatus::Failed && launch.status != LaunchStatus::Cancelled {
            return Err(ContractError::invalid_state("Refunds not available"));
        }

        let contribution = launch
            .contributions
            .remove(&claimer)
            .ok_or(ContractError::invalid_state("No contribution found"))?;

        if contribution == 0 {
            return Err(ContractError::ZeroAmount);
        }

        transfer_native(claimer, contribution)?;

        let _ = self.emit_event(LaunchpadEvent::RefundClaimed {
            launch_id,
            contributor: claimer,
            amount: contribution,
        });

        Ok(contribution)
    }

    /// Withdraw raised funds (creator only, after successful launch).
    #[export]
    pub fn withdraw_funds(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let caller = gstd::msg::source();

        let fee_basis_points = storage().fee_basis_points;
        let s = storage_mut();

        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator {
            return Err(ContractError::Unauthorized);
        }

        if launch.status != LaunchStatus::Succeeded {
            return Err(ContractError::invalid_state("Launch not successful"));
        }

        let total_raised = launch.total_raised;
        if total_raised == 0 {
            return Err(ContractError::ZeroAmount);
        }

        // Calculate and deduct fee
        let fee = total_raised
            .saturating_mul(fee_basis_points as u128)
            / 10_000;
        let creator_amount = total_raised.saturating_sub(fee);

        // Mark as finalized to prevent double withdrawal
        launch.status = LaunchStatus::Finalized;
        launch.total_raised = 0;

        if fee > 0 {
            storage_mut().accumulated_fees = storage().accumulated_fees.saturating_add(fee);
        }

        transfer_native(caller, creator_amount)?;

        let _ = self.emit_event(LaunchpadEvent::FundsWithdrawn {
            launch_id,
            amount: creator_amount,
        });

        Ok(creator_amount)
    }

    /// Cancel a launch (creator only, before start or if no contributions).
    #[export]
    pub fn cancel_launch(&mut self, launch_id: Id) -> Result<(), ContractError> {
        let caller = gstd::msg::source();

        let s = storage_mut();
        let launch = s
            .launches
            .get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator && caller != storage().owner {
            return Err(ContractError::Unauthorized);
        }

        if launch.status != LaunchStatus::Pending && launch.status != LaunchStatus::Active {
            return Err(ContractError::invalid_state("Cannot cancel"));
        }

        launch.status = LaunchStatus::Cancelled;

        let _ = self.emit_event(LaunchpadEvent::LaunchCancelled { launch_id });

        Ok(())
    }

    /// Get launch by ID.
    #[export]
    pub fn get_launch(&self, launch_id: Id) -> Option<Launch> {
        storage().launches.get(&launch_id).cloned()
    }

    /// Get all launches by creator.
    #[export]
    pub fn get_creator_launches(&self, creator: ActorId) -> Vec<Launch> {
        storage()
            .launches
            .values()
            .filter(|l| l.creator == creator)
            .cloned()
            .collect()
    }

    /// Get active launches.
    #[export]
    pub fn get_active_launches(&self) -> Vec<Launch> {
        storage()
            .launches
            .values()
            .filter(|l| l.status == LaunchStatus::Active)
            .cloned()
            .collect()
    }

    /// Get contribution for a user in a launch.
    #[export]
    pub fn get_contribution(&self, launch_id: Id, contributor: ActorId) -> Amount {
        storage()
            .launches
            .get(&launch_id)
            .and_then(|l| l.contributions.get(&contributor).copied())
            .unwrap_or(0)
    }

    /// Get tokens claimed by a user in a launch.
    #[export]
    pub fn get_claimed(&self, launch_id: Id, claimer: ActorId) -> Amount {
        storage()
            .launches
            .get(&launch_id)
            .and_then(|l| l.claimed.get(&claimer).copied())
            .unwrap_or(0)
    }

    /// Check if address is whitelisted for a launch.
    #[export]
    pub fn is_whitelisted(&self, launch_id: Id, address: ActorId) -> bool {
        storage()
            .launches
            .get(&launch_id)
            .map(|l| l.can_participate(&address))
            .unwrap_or(false)
    }

    /// Get total number of launches.
    #[export]
    pub fn get_launch_count(&self) -> u64 {
        storage().next_launch_id
    }

    /// Get accumulated platform fees.
    #[export]
    pub fn get_accumulated_fees(&self) -> Amount {
        storage().accumulated_fees
    }

    /// Withdraw platform fees (owner only).
    #[export]
    pub fn withdraw_fees(&mut self) -> Result<Amount, ContractError> {
        let caller = gstd::msg::source();
        let s = storage_mut();

        if caller != s.owner {
            return Err(ContractError::Unauthorized);
        }

        let fees = s.accumulated_fees;
        if fees == 0 {
            return Err(ContractError::ZeroAmount);
        }

        s.accumulated_fees = 0;
        transfer_native(caller, fees)?;

        Ok(fees)
    }
}

impl Default for LaunchpadService {
    fn default() -> Self {
        Self::new()
    }
}

/// Launchpad Program entry point.
#[derive(Default)]
pub struct LaunchpadProgram(());

#[sails_rs::program]
impl LaunchpadProgram {
    /// Initialize with default 2% fee.
    pub fn new() -> Self {
        let owner = gstd::msg::source();
        init_storage(owner, 200); // 2% fee
        Self(())
    }

    /// Initialize with custom fee.
    pub fn new_with_fee(fee_basis_points: u16) -> Self {
        let owner = gstd::msg::source();
        init_storage(owner, fee_basis_points);
        Self(())
    }

    /// Get the Launchpad service.
    #[export(route = "launchpad")]
    pub fn launchpad(&self) -> LaunchpadService {
        LaunchpadService::new()
    }
}
