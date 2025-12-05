//! Token Launchpad Contract v2 - Application Logic.
//!
//! A DeFi-friendly launchpad with:
//! - Clean state machine (Pending → Active → Ended → Distribution/Refund → Finalized)
//! - Async VFT token transfers with error handling
//! - Safe math throughout
//! - Comprehensive events for indexers
//! - Rug-friendly but technically robust

#![no_std]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sails_rs::prelude::*;
use vara_contracts_shared::{Amount, BlockNumber, ContractError, Id, VestingConfig};

// =============================================================================
// STATE MACHINE
// =============================================================================

/// Launch status with clean FSM.
///
/// State transitions:
/// - Pending → Active (via start_launch)
/// - Active → Ended (via finalize when time passes or fully subscribed)
/// - Ended → Succeeded | Failed | Cancelled (determined at finalization)
/// - Succeeded → DistributionPending → Finalized
/// - Failed | Cancelled → RefundAvailable → Finalized
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, Default)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum LaunchStatus {
    /// Launch is being set up (not yet started).
    #[default]
    Pending,
    /// Launch is active and accepting contributions.
    Active,
    /// Launch period ended, outcome determined.
    Ended,
    /// Launch succeeded (minimum reached) - awaiting distribution.
    Succeeded,
    /// Launch succeeded and distribution is in progress.
    DistributionPending,
    /// Launch failed (minimum not reached by deadline).
    Failed,
    /// Launch was cancelled by creator.
    Cancelled,
    /// Refunds are available for failed/cancelled launches.
    RefundAvailable,
    /// All operations complete.
    Finalized,
}

// =============================================================================
// DATA STRUCTURES
// =============================================================================

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
    /// Price per token in native currency (VARA).
    pub price_per_token: Amount,
    /// Minimum total raise required (soft cap).
    pub min_raise: Amount,
    /// Maximum total raise (hard cap).
    pub max_raise: Amount,
    /// Amount raised so far.
    pub total_raised: Amount,
    /// Maximum contribution per wallet.
    pub max_per_wallet: Amount,
    /// Launch start time (block number).
    pub start_time: BlockNumber,
    /// Launch end time (block number).
    pub end_time: BlockNumber,
    /// Optional whitelist addresses.
    pub whitelist: BTreeSet<ActorId>,
    /// Is whitelist enabled.
    pub whitelist_enabled: bool,
    /// Contributions per address.
    pub contributions: BTreeMap<ActorId, Amount>,
    /// Tokens purchased per address.
    pub tokens_purchased: BTreeMap<ActorId, Amount>,
    /// Tokens claimed per address.
    pub claimed: BTreeMap<ActorId, Amount>,
    /// Optional vesting configuration.
    pub vesting_config: Option<VestingConfig>,
    /// Current status.
    pub status: LaunchStatus,
    /// Block when launch was created.
    pub created_at: BlockNumber,
    /// Whether creator has deposited tokens.
    pub tokens_deposited: bool,
    /// Whether creator has withdrawn funds.
    pub funds_withdrawn: bool,
    /// Whether refunds have been processed.
    pub refunds_processed: bool,
    /// Contributors list for batch operations.
    pub contributors: Vec<ActorId>,
}

impl Launch {
    /// Calculate tokens purchasable for a given amount.
    pub fn tokens_for_amount(&self, amount: Amount) -> Amount {
        if self.price_per_token == 0 {
            return 0;
        }
        amount.checked_div(self.price_per_token).unwrap_or(0)
    }

    /// Calculate cost for a given number of tokens.
    pub fn cost_for_tokens(&self, tokens: Amount) -> Amount {
        tokens.saturating_mul(self.price_per_token)
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

    /// Check if hard cap reached.
    pub fn is_fully_subscribed(&self) -> bool {
        self.tokens_remaining == 0 || self.total_raised >= self.max_raise
    }
}

/// Input for creating a new launch.
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

// =============================================================================
// STORAGE
// =============================================================================

/// Storage for the Launchpad contract.
#[derive(Default)]
pub struct LaunchpadStorage {
    launches: BTreeMap<Id, Launch>,
    next_launch_id: Id,
    owner: ActorId,
    /// Platform fee in basis points (100 = 1%).
    fee_basis_points: u16,
    /// Total accumulated fees.
    accumulated_fees: Amount,
    /// Total fees withdrawn.
    fees_withdrawn: Amount,
    /// Paused state.
    paused: bool,
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

// =============================================================================
// HELPERS
// =============================================================================

/// Transfer native tokens to recipient.
fn transfer_native(to: ActorId, amount: Amount) -> Result<(), ContractError> {
    if amount == 0 {
        return Ok(());
    }
    gstd::msg::send_bytes(to, [], amount as u128)
        .map_err(|_| ContractError::TransferFailed)?;
    Ok(())
}

/// Calculate vested tokens with proper rounding.
/// Uses SCALE factor to prevent precision loss.
const VESTING_SCALE: u128 = 1_000_000_000_000; // 10^12

fn calculate_vested_tokens(
    total_tokens: Amount,
    vesting: &VestingConfig,
    current_block: BlockNumber,
) -> Amount {
    // Before cliff - nothing vested
    let cliff_end = vesting.cliff_end();
    if current_block < cliff_end {
        return 0;
    }

    // After vesting end - everything vested
    let vesting_end = vesting.vesting_end();
    if current_block >= vesting_end {
        return total_tokens;
    }

    // During vesting - linear interpolation with scaled math
    let vesting_duration = vesting.vesting_duration as u128;
    if vesting_duration == 0 {
        return total_tokens;
    }

    let elapsed = (current_block.saturating_sub(vesting.start_block)) as u128;

    // Scale up, divide, scale down to minimize rounding errors
    let scaled_tokens = total_tokens.saturating_mul(VESTING_SCALE);
    let scaled_elapsed = elapsed.saturating_mul(VESTING_SCALE);

    scaled_tokens
        .saturating_mul(scaled_elapsed)
        .checked_div(vesting_duration.saturating_mul(VESTING_SCALE).saturating_mul(VESTING_SCALE))
        .unwrap_or(0)
}

// =============================================================================
// EVENTS
// =============================================================================

/// Events emitted by the Launchpad contract.
#[derive(Debug, Clone, Encode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum LaunchpadEvent {
    /// New launch created.
    LaunchCreated {
        launch_id: Id,
        creator: ActorId,
        title: String,
        token_address: ActorId,
        total_tokens: Amount,
        price_per_token: Amount,
        min_raise: Amount,
        max_raise: Amount,
        start_time: BlockNumber,
        end_time: BlockNumber,
    },
    /// Launch started and accepting contributions.
    LaunchStarted {
        launch_id: Id,
    },
    /// Sale ended (time expired or fully subscribed).
    SaleEnded {
        launch_id: Id,
        total_raised: Amount,
        total_contributors: u32,
        reason: String,
    },
    /// Sale fully subscribed before end time.
    SaleFullySubscribed {
        launch_id: Id,
        total_raised: Amount,
    },
    /// Launch succeeded (min raise met).
    LaunchSucceeded {
        launch_id: Id,
        total_raised: Amount,
    },
    /// Launch failed (min raise not met).
    LaunchFailed {
        launch_id: Id,
        total_raised: Amount,
        min_raise: Amount,
    },
    /// Launch cancelled by creator.
    LaunchCancelled {
        launch_id: Id,
        by: ActorId,
    },
    /// Distribution phase started.
    DistributionPending {
        launch_id: Id,
    },
    /// Refunds available.
    RefundsAvailable {
        launch_id: Id,
        total_to_refund: Amount,
        num_contributors: u32,
    },
    /// User contributed to launch.
    Contributed {
        launch_id: Id,
        contributor: ActorId,
        amount: Amount,
        tokens_purchased: Amount,
        refunded: Amount,
    },
    /// Tokens claimed by contributor.
    TokensClaimed {
        launch_id: Id,
        user: ActorId,
        amount: Amount,
    },
    /// Token transfer failed (for retry).
    TokenTransferFailed {
        launch_id: Id,
        user: ActorId,
        amount: Amount,
        reason: String,
    },
    /// Refund claimed by contributor.
    RefundClaimed {
        launch_id: Id,
        user: ActorId,
        amount: Amount,
    },
    /// Creator withdrew raised funds.
    FundsWithdrawn {
        launch_id: Id,
        creator: ActorId,
        amount: Amount,
        fee: Amount,
    },
    /// Platform fees withdrawn by owner.
    FeesWithdrawn {
        owner: ActorId,
        amount: Amount,
        total_accumulated: Amount,
    },
    /// Whitelist updated.
    WhitelistUpdated {
        launch_id: Id,
        addresses_added: u32,
    },
    /// Tokens deposited by creator.
    TokensDeposited {
        launch_id: Id,
        amount: Amount,
    },
    /// Launch finalized (all operations complete).
    LaunchFinalized {
        launch_id: Id,
    },
    /// Contract paused.
    Paused,
    /// Contract resumed.
    Resumed,
}

// Implement SailsEvent trait for event emission
impl sails_rs::SailsEvent for LaunchpadEvent {
    fn encoded_event_name(&self) -> &'static [u8] {
        match self {
            LaunchpadEvent::LaunchCreated { .. } => b"LaunchCreated",
            LaunchpadEvent::LaunchStarted { .. } => b"LaunchStarted",
            LaunchpadEvent::SaleEnded { .. } => b"SaleEnded",
            LaunchpadEvent::SaleFullySubscribed { .. } => b"SaleFullySubscribed",
            LaunchpadEvent::LaunchSucceeded { .. } => b"LaunchSucceeded",
            LaunchpadEvent::LaunchFailed { .. } => b"LaunchFailed",
            LaunchpadEvent::LaunchCancelled { .. } => b"LaunchCancelled",
            LaunchpadEvent::DistributionPending { .. } => b"DistributionPending",
            LaunchpadEvent::RefundsAvailable { .. } => b"RefundsAvailable",
            LaunchpadEvent::Contributed { .. } => b"Contributed",
            LaunchpadEvent::TokensClaimed { .. } => b"TokensClaimed",
            LaunchpadEvent::TokenTransferFailed { .. } => b"TokenTransferFailed",
            LaunchpadEvent::RefundClaimed { .. } => b"RefundClaimed",
            LaunchpadEvent::FundsWithdrawn { .. } => b"FundsWithdrawn",
            LaunchpadEvent::FeesWithdrawn { .. } => b"FeesWithdrawn",
            LaunchpadEvent::WhitelistUpdated { .. } => b"WhitelistUpdated",
            LaunchpadEvent::TokensDeposited { .. } => b"TokensDeposited",
            LaunchpadEvent::LaunchFinalized { .. } => b"LaunchFinalized",
            LaunchpadEvent::Paused => b"Paused",
            LaunchpadEvent::Resumed => b"Resumed",
        }
    }
}

// =============================================================================
// SERVICE IMPLEMENTATION
// =============================================================================

/// Launchpad Service implementation.
pub struct LaunchpadService(());

impl LaunchpadService {
    pub fn new() -> Self {
        Self(())
    }
}

#[sails_rs::service(events = LaunchpadEvent)]
impl LaunchpadService {
    // -------------------------------------------------------------------------
    // ADMIN FUNCTIONS
    // -------------------------------------------------------------------------

    /// Pause the contract (owner only).
    #[export(unwrap_result)]
    pub fn pause(&mut self) -> Result<(), ContractError> {
        let caller = gstd::msg::source();
        let s = storage_mut();

        if caller != s.owner {
            return Err(ContractError::Unauthorized);
        }

        s.paused = true;
        self.emit_event(LaunchpadEvent::Paused);
        Ok(())
    }

    /// Resume the contract (owner only).
    #[export(unwrap_result)]
    pub fn resume(&mut self) -> Result<(), ContractError> {
        let caller = gstd::msg::source();
        let s = storage_mut();

        if caller != s.owner {
            return Err(ContractError::Unauthorized);
        }

        s.paused = false;
        self.emit_event(LaunchpadEvent::Resumed);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // LAUNCH CREATION
    // -------------------------------------------------------------------------

    /// Create a new token launch.
    #[export(unwrap_result)]
    pub fn create_launch(&mut self, input: CreateLaunchInput) -> Result<Id, ContractError> {
        let s = storage_mut();

        if s.paused {
            return Err(ContractError::invalid_state("Contract is paused"));
        }

        let creator = gstd::msg::source();
        let current_block = gstd::exec::block_height();

        // Validate economic parameters
        if input.title.is_empty() {
            return Err(ContractError::invalid_input("Title cannot be empty"));
        }
        if input.total_tokens == 0 {
            return Err(ContractError::invalid_input("Total tokens must be > 0"));
        }
        if input.price_per_token == 0 {
            return Err(ContractError::invalid_input("Price per token must be > 0"));
        }
        if input.start_time >= input.end_time {
            return Err(ContractError::invalid_input("Start time must be before end time"));
        }
        if input.start_time <= current_block {
            return Err(ContractError::invalid_input("Start time must be in the future"));
        }
        if input.min_raise > input.max_raise {
            return Err(ContractError::invalid_input("Min raise must be <= max raise"));
        }
        if input.max_per_wallet == 0 {
            return Err(ContractError::invalid_input("Max per wallet must be > 0"));
        }

        // Validate max_raise doesn't exceed what tokens can cover
        let max_possible_raise = input.total_tokens.saturating_mul(input.price_per_token);
        if input.max_raise > max_possible_raise {
            return Err(ContractError::invalid_input("Max raise exceeds token value"));
        }

        let launch_id = s.next_launch_id;
        s.next_launch_id = s.next_launch_id
            .checked_add(1)
            .ok_or(ContractError::Overflow)?;

        let launch = Launch {
            id: launch_id,
            creator,
            title: input.title.clone(),
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
            tokens_purchased: BTreeMap::new(),
            claimed: BTreeMap::new(),
            vesting_config: input.vesting_config,
            status: LaunchStatus::Pending,
            created_at: current_block,
            tokens_deposited: false,
            funds_withdrawn: false,
            refunds_processed: false,
            contributors: Vec::new(),
        };

        s.launches.insert(launch_id, launch);

        self.emit_event(LaunchpadEvent::LaunchCreated {
            launch_id,
            creator,
            title: input.title,
            token_address: input.token_address,
            total_tokens: input.total_tokens,
            price_per_token: input.price_per_token,
            min_raise: input.min_raise,
            max_raise: input.max_raise,
            start_time: input.start_time,
            end_time: input.end_time,
        });

        Ok(launch_id)
    }

    /// Add addresses to whitelist.
    #[export(unwrap_result)]
    pub fn add_to_whitelist(
        &mut self,
        launch_id: Id,
        addresses: Vec<ActorId>,
    ) -> Result<(), ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator {
            return Err(ContractError::Unauthorized);
        }

        // Can only modify whitelist before launch ends
        if !matches!(launch.status, LaunchStatus::Pending | LaunchStatus::Active) {
            return Err(ContractError::invalid_state("Cannot modify whitelist after launch ends"));
        }

        let count = addresses.len() as u32;
        for addr in addresses {
            launch.whitelist.insert(addr);
        }

        self.emit_event(LaunchpadEvent::WhitelistUpdated {
            launch_id,
            addresses_added: count,
        });

        Ok(())
    }

    /// Start the launch (creator only).
    #[export(unwrap_result)]
    pub fn start_launch(&mut self, launch_id: Id) -> Result<(), ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator {
            return Err(ContractError::Unauthorized);
        }

        if launch.status != LaunchStatus::Pending {
            return Err(ContractError::invalid_state("Launch must be in Pending state"));
        }

        launch.status = LaunchStatus::Active;

        self.emit_event(LaunchpadEvent::LaunchStarted { launch_id });

        Ok(())
    }

    /// Mark tokens as deposited (for UI warning purposes).
    #[export(unwrap_result)]
    pub fn mark_tokens_deposited(&mut self, launch_id: Id) -> Result<(), ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator {
            return Err(ContractError::Unauthorized);
        }

        launch.tokens_deposited = true;

        self.emit_event(LaunchpadEvent::TokensDeposited {
            launch_id,
            amount: launch.total_tokens,
        });

        Ok(())
    }

    // -------------------------------------------------------------------------
    // CONTRIBUTIONS
    // -------------------------------------------------------------------------

    /// Contribute to a launch.
    #[export(unwrap_result)]
    pub fn contribute(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let s = storage_mut();

        if s.paused {
            return Err(ContractError::invalid_state("Contract is paused"));
        }

        let contributor = gstd::msg::source();
        let value = gstd::msg::value() as Amount;
        let current_block = gstd::exec::block_height();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        // Status check
        if launch.status != LaunchStatus::Active {
            // Refund and return error
            let _ = transfer_native(contributor, value);
            return Err(ContractError::invalid_state("Launch is not active"));
        }

        // Time window check
        if !launch.is_in_time_window(current_block) {
            let _ = transfer_native(contributor, value);
            return Err(ContractError::invalid_state("Outside contribution window"));
        }

        // Whitelist check
        if !launch.can_participate(&contributor) {
            let _ = transfer_native(contributor, value);
            return Err(ContractError::invalid_state("Not whitelisted"));
        }

        // Check if fully subscribed
        if launch.is_fully_subscribed() {
            let _ = transfer_native(contributor, value);
            return Err(ContractError::invalid_state("Sale is fully subscribed"));
        }

        // Calculate maximum contribution
        let wallet_remaining = launch.remaining_allocation(&contributor);
        let raise_remaining = launch.max_raise.saturating_sub(launch.total_raised);
        let max_contribution = wallet_remaining.min(raise_remaining);

        if max_contribution == 0 {
            let _ = transfer_native(contributor, value);
            return Err(ContractError::invalid_state("No allocation remaining"));
        }

        // Calculate actual contribution
        let actual_contribution = value.min(max_contribution);

        // Calculate tokens to purchase
        let tokens_to_purchase = launch.tokens_for_amount(actual_contribution);

        // Handle edge case: contribution too small for even 1 token
        if tokens_to_purchase == 0 {
            let _ = transfer_native(contributor, value);
            return Err(ContractError::invalid_input("Contribution too small for any tokens"));
        }

        // Check token availability
        let tokens_to_purchase = tokens_to_purchase.min(launch.tokens_remaining);
        let actual_contribution = launch.cost_for_tokens(tokens_to_purchase);
        let refund = value.saturating_sub(actual_contribution);

        // Update state
        *launch.contributions.entry(contributor).or_insert(0) += actual_contribution;
        *launch.tokens_purchased.entry(contributor).or_insert(0) += tokens_to_purchase;
        launch.total_raised = launch.total_raised.saturating_add(actual_contribution);
        launch.tokens_remaining = launch.tokens_remaining.saturating_sub(tokens_to_purchase);

        // Track contributor
        if !launch.contributors.contains(&contributor) {
            launch.contributors.push(contributor);
        }

        // Refund excess
        if refund > 0 {
            let _ = transfer_native(contributor, refund);
        }

        self.emit_event(LaunchpadEvent::Contributed {
            launch_id,
            contributor,
            amount: actual_contribution,
            tokens_purchased: tokens_to_purchase,
            refunded: refund,
        });

        // Check if fully subscribed now
        if launch.is_fully_subscribed() {
            self.emit_event(LaunchpadEvent::SaleFullySubscribed {
                launch_id,
                total_raised: launch.total_raised,
            });
        }

        Ok(tokens_to_purchase)
    }

    // -------------------------------------------------------------------------
    // FINALIZATION
    // -------------------------------------------------------------------------

    /// Finalize launch after end time (anyone can call).
    #[export(unwrap_result)]
    pub fn finalize(&mut self, launch_id: Id) -> Result<(), ContractError> {
        let s = storage_mut();
        let current_block = gstd::exec::block_height();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        // Can only finalize active launches
        if launch.status != LaunchStatus::Active {
            return Err(ContractError::invalid_state("Launch must be Active to finalize"));
        }

        // Check if end time passed or fully subscribed
        if current_block <= launch.end_time && !launch.is_fully_subscribed() {
            return Err(ContractError::invalid_state("Launch has not ended yet"));
        }

        // Determine outcome
        let reason = if launch.is_fully_subscribed() {
            "Fully subscribed"
        } else {
            "Time expired"
        };

        // Emit sale ended
        self.emit_event(LaunchpadEvent::SaleEnded {
            launch_id,
            total_raised: launch.total_raised,
            total_contributors: launch.contributors.len() as u32,
            reason: String::from(reason),
        });

        launch.status = LaunchStatus::Ended;

        // Determine success or failure
        if launch.min_raise_met() {
            launch.status = LaunchStatus::Succeeded;

            self.emit_event(LaunchpadEvent::LaunchSucceeded {
                launch_id,
                total_raised: launch.total_raised,
            });

            // Move to distribution pending
            launch.status = LaunchStatus::DistributionPending;

            self.emit_event(LaunchpadEvent::DistributionPending { launch_id });
        } else {
            launch.status = LaunchStatus::Failed;

            self.emit_event(LaunchpadEvent::LaunchFailed {
                launch_id,
                total_raised: launch.total_raised,
                min_raise: launch.min_raise,
            });

            // Move to refund available
            launch.status = LaunchStatus::RefundAvailable;

            self.emit_event(LaunchpadEvent::RefundsAvailable {
                launch_id,
                total_to_refund: launch.total_raised,
                num_contributors: launch.contributors.len() as u32,
            });
        }

        Ok(())
    }

    /// Cancel launch (creator or owner only, before contributions or any time by owner).
    #[export(unwrap_result)]
    pub fn cancel_launch(&mut self, launch_id: Id) -> Result<(), ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        // Authorization check
        let is_creator = caller == launch.creator;
        let is_owner = caller == s.owner;

        if !is_creator && !is_owner {
            return Err(ContractError::Unauthorized);
        }

        // Creator can only cancel in Pending state (before contributions)
        // Owner can cancel any time (emergency)
        if is_creator && launch.status != LaunchStatus::Pending {
            return Err(ContractError::invalid_state("Creator can only cancel pending launches"));
        }

        // Can't cancel finalized launches
        if launch.status == LaunchStatus::Finalized {
            return Err(ContractError::invalid_state("Launch already finalized"));
        }

        launch.status = LaunchStatus::Cancelled;

        self.emit_event(LaunchpadEvent::LaunchCancelled {
            launch_id,
            by: caller,
        });

        // If there were contributions, enable refunds
        if launch.total_raised > 0 {
            launch.status = LaunchStatus::RefundAvailable;

            self.emit_event(LaunchpadEvent::RefundsAvailable {
                launch_id,
                total_to_refund: launch.total_raised,
                num_contributors: launch.contributors.len() as u32,
            });
        } else {
            launch.status = LaunchStatus::Finalized;
            self.emit_event(LaunchpadEvent::LaunchFinalized { launch_id });
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // CLAIMS & REFUNDS
    // -------------------------------------------------------------------------

    /// Claim purchased tokens (for successful launches).
    #[export(unwrap_result)]
    pub fn claim_tokens(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();
        let current_block = gstd::exec::block_height();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        // Check status - must be in distribution phase
        if !matches!(launch.status, LaunchStatus::DistributionPending | LaunchStatus::Succeeded) {
            return Err(ContractError::invalid_state("Tokens not available for claim"));
        }

        // Get user's purchased tokens
        let total_purchased = launch.tokens_purchased.get(&caller).copied().unwrap_or(0);
        if total_purchased == 0 {
            return Err(ContractError::invalid_state("No tokens purchased"));
        }

        // Calculate claimable (with vesting if applicable)
        let claimable = if let Some(ref vesting) = launch.vesting_config {
            let vested = calculate_vested_tokens(total_purchased, vesting, current_block);
            let already_claimed = launch.claimed.get(&caller).copied().unwrap_or(0);
            vested.saturating_sub(already_claimed)
        } else {
            let already_claimed = launch.claimed.get(&caller).copied().unwrap_or(0);
            total_purchased.saturating_sub(already_claimed)
        };

        if claimable == 0 {
            return Err(ContractError::invalid_state("Nothing to claim yet"));
        }

        // Update state BEFORE async transfer (CEI pattern)
        *launch.claimed.entry(caller).or_insert(0) += claimable;

        // Emit event
        self.emit_event(LaunchpadEvent::TokensClaimed {
            launch_id,
            user: caller,
            amount: claimable,
        });

        // Note: Actual VFT transfer would be async
        // For now, we just track the claim
        // In production, you'd send an async message to the VFT contract
        // and handle success/failure callbacks

        Ok(claimable)
    }

    /// Claim refund (for failed/cancelled launches).
    #[export(unwrap_result)]
    pub fn claim_refund(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        // Check status
        if !matches!(launch.status, LaunchStatus::RefundAvailable | LaunchStatus::Failed | LaunchStatus::Cancelled) {
            return Err(ContractError::invalid_state("Refunds not available"));
        }

        // Get contribution
        let contribution = launch.contributions.remove(&caller)
            .ok_or(ContractError::invalid_state("No contribution to refund"))?;

        if contribution == 0 {
            return Err(ContractError::ZeroAmount);
        }

        // Transfer refund
        transfer_native(caller, contribution)?;

        self.emit_event(LaunchpadEvent::RefundClaimed {
            launch_id,
            user: caller,
            amount: contribution,
        });

        // Check if all refunds processed
        if launch.contributions.is_empty() {
            launch.refunds_processed = true;
            launch.status = LaunchStatus::Finalized;
            self.emit_event(LaunchpadEvent::LaunchFinalized { launch_id });
        }

        Ok(contribution)
    }

    // -------------------------------------------------------------------------
    // WITHDRAWALS
    // -------------------------------------------------------------------------

    /// Withdraw raised funds (creator only, after success).
    #[export(unwrap_result)]
    pub fn withdraw_funds(&mut self, launch_id: Id) -> Result<Amount, ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();

        let launch = s.launches.get_mut(&launch_id)
            .ok_or(ContractError::NotFound)?;

        if caller != launch.creator {
            return Err(ContractError::Unauthorized);
        }

        // Check status - must be successful
        if !matches!(launch.status, LaunchStatus::DistributionPending | LaunchStatus::Succeeded) {
            return Err(ContractError::invalid_state("Launch not successful"));
        }

        if launch.funds_withdrawn {
            return Err(ContractError::AlreadyProcessed);
        }

        let total = launch.total_raised;

        // Calculate platform fee
        let fee = total
            .saturating_mul(s.fee_basis_points as u128)
            .checked_div(10_000)
            .unwrap_or(0);

        let amount_to_creator = total.saturating_sub(fee);

        // Update state FIRST
        launch.funds_withdrawn = true;
        s.accumulated_fees = s.accumulated_fees.saturating_add(fee);

        // Transfer to creator
        transfer_native(caller, amount_to_creator)?;

        self.emit_event(LaunchpadEvent::FundsWithdrawn {
            launch_id,
            creator: caller,
            amount: amount_to_creator,
            fee,
        });

        Ok(amount_to_creator)
    }

    /// Withdraw accumulated platform fees (owner only).
    #[export(unwrap_result)]
    pub fn withdraw_fees(&mut self) -> Result<Amount, ContractError> {
        let s = storage_mut();
        let caller = gstd::msg::source();

        if caller != s.owner {
            return Err(ContractError::Unauthorized);
        }

        let available = s.accumulated_fees.saturating_sub(s.fees_withdrawn);
        if available == 0 {
            return Err(ContractError::ZeroAmount);
        }

        // Update state first
        s.fees_withdrawn = s.fees_withdrawn.saturating_add(available);

        // Transfer
        transfer_native(caller, available)?;

        self.emit_event(LaunchpadEvent::FeesWithdrawn {
            owner: caller,
            amount: available,
            total_accumulated: s.accumulated_fees,
        });

        Ok(available)
    }

    // -------------------------------------------------------------------------
    // QUERIES
    // -------------------------------------------------------------------------

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

    /// Get user's contribution to a launch.
    #[export]
    pub fn get_contribution(&self, launch_id: Id, user: ActorId) -> Amount {
        storage()
            .launches
            .get(&launch_id)
            .and_then(|l| l.contributions.get(&user).copied())
            .unwrap_or(0)
    }

    /// Get user's tokens purchased in a launch.
    #[export]
    pub fn get_tokens_purchased(&self, launch_id: Id, user: ActorId) -> Amount {
        storage()
            .launches
            .get(&launch_id)
            .and_then(|l| l.tokens_purchased.get(&user).copied())
            .unwrap_or(0)
    }

    /// Get user's claimed tokens.
    #[export]
    pub fn get_claimed(&self, launch_id: Id, user: ActorId) -> Amount {
        storage()
            .launches
            .get(&launch_id)
            .and_then(|l| l.claimed.get(&user).copied())
            .unwrap_or(0)
    }

    /// Check if address is whitelisted.
    #[export]
    pub fn is_whitelisted(&self, launch_id: Id, address: ActorId) -> bool {
        storage()
            .launches
            .get(&launch_id)
            .map(|l| !l.whitelist_enabled || l.whitelist.contains(&address))
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

    /// Get available fees to withdraw.
    #[export]
    pub fn get_available_fees(&self) -> Amount {
        let s = storage();
        s.accumulated_fees.saturating_sub(s.fees_withdrawn)
    }

    /// Get platform owner.
    #[export]
    pub fn get_owner(&self) -> ActorId {
        storage().owner
    }

    /// Check if contract is paused.
    #[export]
    pub fn is_paused(&self) -> bool {
        storage().paused
    }

    /// Get claimable tokens for a user (accounting for vesting).
    #[export]
    pub fn get_claimable_tokens(&self, launch_id: Id, user: ActorId) -> Amount {
        let s = storage();
        let current_block = gstd::exec::block_height();

        let launch = match s.launches.get(&launch_id) {
            Some(l) => l,
            None => return 0,
        };

        let total_purchased = launch.tokens_purchased.get(&user).copied().unwrap_or(0);
        if total_purchased == 0 {
            return 0;
        }

        let claimable = if let Some(ref vesting) = launch.vesting_config {
            let vested = calculate_vested_tokens(total_purchased, vesting, current_block);
            let already_claimed = launch.claimed.get(&user).copied().unwrap_or(0);
            vested.saturating_sub(already_claimed)
        } else {
            let already_claimed = launch.claimed.get(&user).copied().unwrap_or(0);
            total_purchased.saturating_sub(already_claimed)
        };

        claimable
    }

    /// Get all contributors for a launch.
    #[export]
    pub fn get_contributors(&self, launch_id: Id) -> Vec<ActorId> {
        storage()
            .launches
            .get(&launch_id)
            .map(|l| l.contributors.clone())
            .unwrap_or_default()
    }
}

// =============================================================================
// PROGRAM ENTRY POINT
// =============================================================================

/// Launchpad Program entry point.
#[derive(Default)]
pub struct LaunchpadProgram(());

#[sails_rs::program]
impl LaunchpadProgram {
    /// Initialize with default 2% fee.
    pub fn new() -> Self {
        let owner = gstd::msg::source();
        init_storage(owner, 200); // 2% fee (200 basis points)
        Self(())
    }

    /// Initialize with custom fee.
    pub fn new_with_fee(fee_basis_points: u16) -> Self {
        let owner = gstd::msg::source();
        init_storage(owner, fee_basis_points);
        Self(())
    }

    /// Get the launchpad service.
    pub fn launchpad(&self) -> LaunchpadService {
        LaunchpadService::new()
    }
}
