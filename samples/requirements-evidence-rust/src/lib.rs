//! An executable example of tracing requirements into Rust types and evidence.
//!
//! Untrusted bytes must become a [`ParsedTransfer`], then an actor must obtain an
//! `AuthorizedTransfer` from a ledger before the transfer can execute.
//!
//! ```compile_fail
//! use requirements_evidence_rust::ParsedTransfer;
//!
//! let parsed = ParsedTransfer::parse(b"7,10,20,500").unwrap();
//! parsed.execute(); // Parsing alone does not grant authority to execute.
//! ```

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::{collections::BTreeMap, num::NonZeroU64};

/// Maximum accepted size of the untrusted comma-separated input.
pub const MAX_TRANSFER_INPUT_BYTES: usize = 128;

/// Maximum amount accepted by one transfer.
pub const MAX_TRANSFER_AMOUNT: u64 = 1_000_000;

/// The stable identifier of an authenticated actor.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ActorId(NonZeroU64);

impl ActorId {
    /// Creates an actor identifier, rejecting zero.
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the integer representation.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// The stable identifier of an account.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AccountId(NonZeroU64);

impl AccountId {
    /// Creates an account identifier, rejecting zero.
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the integer representation.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// The idempotency identifier of a transfer request.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RequestId(NonZeroU64);

impl RequestId {
    /// Creates a request identifier, rejecting zero.
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the integer representation.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// A non-zero transfer amount within the business-rule limit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TransferAmount(NonZeroU64);

impl TransferAmount {
    /// Creates an amount in the inclusive range `1..=MAX_TRANSFER_AMOUNT`.
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) if value.get() <= MAX_TRANSFER_AMOUNT => Some(Self(value)),
            Some(_) | None => None,
        }
    }

    /// Returns the number of units to transfer.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// A transfer that passed syntax, identifier, amount, and account validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParsedTransfer {
    request_id: RequestId,
    source: AccountId,
    destination: AccountId,
    amount: TransferAmount,
}

/// Errors produced while parsing untrusted transfer bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseTransferError {
    /// The byte input exceeds [`MAX_TRANSFER_INPUT_BYTES`].
    InputTooLong,
    /// The input is not valid UTF-8.
    NotUtf8,
    /// The input does not contain exactly four comma-separated fields.
    WrongFieldCount,
    /// The request identifier is not a non-zero decimal `u64`.
    InvalidRequestId,
    /// The source account identifier is not a non-zero decimal `u64`.
    InvalidSourceAccountId,
    /// The destination account identifier is not a non-zero decimal `u64`.
    InvalidDestinationAccountId,
    /// The amount is not a decimal integer in `1..=MAX_TRANSFER_AMOUNT`.
    InvalidAmount,
    /// The source and destination refer to the same account.
    SameAccount,
}

impl ParsedTransfer {
    /// Creates a transfer from validated values while requiring distinct accounts.
    pub const fn new(
        request_id: RequestId,
        source: AccountId,
        destination: AccountId,
        amount: TransferAmount,
    ) -> Result<Self, ParseTransferError> {
        if source.get() == destination.get() {
            return Err(ParseTransferError::SameAccount);
        }

        Ok(Self {
            request_id,
            source,
            destination,
            amount,
        })
    }

    /// Parses `request_id,source_account,destination_account,amount`.
    ///
    /// Length is checked before UTF-8 decoding or numeric parsing.
    pub fn parse(input: &[u8]) -> Result<Self, ParseTransferError> {
        if input.len() > MAX_TRANSFER_INPUT_BYTES {
            return Err(ParseTransferError::InputTooLong);
        }

        let input = std::str::from_utf8(input).map_err(|_| ParseTransferError::NotUtf8)?;
        let mut fields = input.split(',');
        let request_id = fields.next().ok_or(ParseTransferError::WrongFieldCount)?;
        let source = fields.next().ok_or(ParseTransferError::WrongFieldCount)?;
        let destination = fields.next().ok_or(ParseTransferError::WrongFieldCount)?;
        let amount = fields.next().ok_or(ParseTransferError::WrongFieldCount)?;

        if fields.next().is_some() {
            return Err(ParseTransferError::WrongFieldCount);
        }

        let request_id = request_id
            .parse()
            .ok()
            .and_then(RequestId::new)
            .ok_or(ParseTransferError::InvalidRequestId)?;
        let source = source
            .parse()
            .ok()
            .and_then(AccountId::new)
            .ok_or(ParseTransferError::InvalidSourceAccountId)?;
        let destination = destination
            .parse()
            .ok()
            .and_then(AccountId::new)
            .ok_or(ParseTransferError::InvalidDestinationAccountId)?;
        let amount = amount
            .parse()
            .ok()
            .and_then(TransferAmount::new)
            .ok_or(ParseTransferError::InvalidAmount)?;

        Self::new(request_id, source, destination, amount)
    }

    /// Returns the idempotency identifier.
    pub const fn request_id(self) -> RequestId {
        self.request_id
    }

    /// Returns the source account identifier.
    pub const fn source(self) -> AccountId {
        self.source
    }

    /// Returns the destination account identifier.
    pub const fn destination(self) -> AccountId {
        self.destination
    }

    /// Returns the validated transfer amount.
    pub const fn amount(self) -> TransferAmount {
        self.amount
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Account {
    owner: ActorId,
    balance: u64,
}

/// Errors produced while constructing a ledger fixture or restored ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LedgerBuildError {
    /// Two input rows use the same account identifier.
    DuplicateAccount(AccountId),
}

/// A single-process ledger that owns accounts and completed request IDs.
#[derive(Debug)]
pub struct Ledger {
    accounts: BTreeMap<AccountId, Account>,
    completed: BTreeMap<RequestId, ParsedTransfer>,
}

impl Ledger {
    /// Builds a ledger from `(account_id, owner_id, opening_balance)` rows.
    pub fn new(accounts: &[(AccountId, ActorId, u64)]) -> Result<Self, LedgerBuildError> {
        let mut ledger = Self {
            accounts: BTreeMap::new(),
            completed: BTreeMap::new(),
        };

        for &(id, owner, balance) in accounts {
            if ledger
                .accounts
                .insert(id, Account { owner, balance })
                .is_some()
            {
                return Err(LedgerBuildError::DuplicateAccount(id));
            }
        }

        Ok(ledger)
    }

    /// Returns the current balance, or `None` when the account is absent.
    pub fn balance(&self, account: AccountId) -> Option<u64> {
        self.accounts.get(&account).map(|account| account.balance)
    }

    /// Verifies that `actor` owns the source account and borrows the ledger until execution.
    pub fn authorize(
        &mut self,
        actor: ActorId,
        transfer: ParsedTransfer,
    ) -> Result<AuthorizedTransfer<'_>, TransferError> {
        let source = self
            .accounts
            .get(&transfer.source())
            .ok_or(TransferError::SourceAccountNotFound)?;
        if source.owner != actor {
            return Err(TransferError::UnauthorizedSource);
        }
        if !self.accounts.contains_key(&transfer.destination()) {
            return Err(TransferError::DestinationAccountNotFound);
        }

        Ok(AuthorizedTransfer {
            ledger: self,
            transfer,
        })
    }
}

/// A transfer that owns an exclusive borrow of the ledger after authorization.
#[derive(Debug)]
pub struct AuthorizedTransfer<'ledger> {
    ledger: &'ledger mut Ledger,
    transfer: ParsedTransfer,
}

impl AuthorizedTransfer<'_> {
    /// Applies a new transfer once, or reports a safe replay.
    pub fn execute(self) -> Result<TransferReceipt, TransferError> {
        let request_id = self.transfer.request_id();
        if let Some(completed) = self.ledger.completed.get(&request_id) {
            return if *completed == self.transfer {
                Ok(TransferReceipt {
                    request_id,
                    disposition: Disposition::Replayed,
                })
            } else {
                Err(TransferError::RequestIdConflict)
            };
        }

        let source = *self
            .ledger
            .accounts
            .get(&self.transfer.source())
            .ok_or(TransferError::SourceAccountNotFound)?;
        let destination = *self
            .ledger
            .accounts
            .get(&self.transfer.destination())
            .ok_or(TransferError::DestinationAccountNotFound)?;
        let (source_balance, destination_balance) =
            transition(source.balance, destination.balance, self.transfer.amount())?;

        self.ledger.accounts.insert(
            self.transfer.source(),
            Account {
                balance: source_balance,
                ..source
            },
        );
        self.ledger.accounts.insert(
            self.transfer.destination(),
            Account {
                balance: destination_balance,
                ..destination
            },
        );
        self.ledger.completed.insert(request_id, self.transfer);

        Ok(TransferReceipt {
            request_id,
            disposition: Disposition::Applied,
        })
    }
}

fn transition(
    source_balance: u64,
    destination_balance: u64,
    amount: TransferAmount,
) -> Result<(u64, u64), TransferError> {
    let source_balance = source_balance
        .checked_sub(amount.get())
        .ok_or(TransferError::InsufficientFunds)?;
    let destination_balance = destination_balance
        .checked_add(amount.get())
        .ok_or(TransferError::DestinationOverflow)?;
    Ok((source_balance, destination_balance))
}

/// The observable outcome of a successful execution request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferReceipt {
    request_id: RequestId,
    disposition: Disposition,
}

impl TransferReceipt {
    /// Returns the executed or replayed request identifier.
    pub const fn request_id(self) -> RequestId {
        self.request_id
    }

    /// Returns whether this call applied state or replayed a prior result.
    pub const fn disposition(self) -> Disposition {
        self.disposition
    }
}

/// Whether a successful execution call changed ledger state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// The request changed both account balances.
    Applied,
    /// An identical completed request returned without changing balances.
    Replayed,
}

/// Errors from authorization or execution of a parsed transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferError {
    /// The source account does not exist.
    SourceAccountNotFound,
    /// The destination account does not exist.
    DestinationAccountNotFound,
    /// The actor does not own the source account.
    UnauthorizedSource,
    /// The source account cannot cover the transfer amount.
    InsufficientFunds,
    /// The destination balance would exceed `u64::MAX`.
    DestinationOverflow,
    /// A completed request ID was reused with different contents.
    RequestIdConflict,
}

#[cfg(kani)]
mod verification {
    use super::{MAX_TRANSFER_AMOUNT, TransferAmount, TransferError, transition};

    #[kani::proof]
    fn successful_transition_preserves_total() {
        let source = kani::any::<u64>();
        let destination = kani::any::<u64>();
        let raw_amount = kani::any::<u64>();
        kani::assume((1..=MAX_TRANSFER_AMOUNT).contains(&raw_amount));
        kani::assume(source >= raw_amount);
        kani::assume(destination <= u64::MAX - raw_amount);

        let Some(amount) = TransferAmount::new(raw_amount) else {
            unreachable!();
        };
        let Ok((new_source, new_destination)) = transition(source, destination, amount) else {
            unreachable!();
        };

        assert_eq!(new_source, source - raw_amount);
        assert_eq!(new_destination, destination + raw_amount);
        assert_eq!(
            u128::from(new_source) + u128::from(new_destination),
            u128::from(source) + u128::from(destination)
        );
    }

    #[kani::proof]
    fn transition_result_matches_preconditions() {
        let source = kani::any::<u64>();
        let destination = kani::any::<u64>();
        let raw_amount = kani::any::<u64>();
        kani::assume((1..=MAX_TRANSFER_AMOUNT).contains(&raw_amount));

        let Some(amount) = TransferAmount::new(raw_amount) else {
            unreachable!();
        };
        match transition(source, destination, amount) {
            Ok((new_source, new_destination)) => {
                assert!(source >= raw_amount);
                assert!(destination <= u64::MAX - raw_amount);
                assert_eq!(new_source, source - raw_amount);
                assert_eq!(new_destination, destination + raw_amount);
            }
            Err(TransferError::InsufficientFunds) => assert!(source < raw_amount),
            Err(TransferError::DestinationOverflow) => {
                assert!(source >= raw_amount);
                assert!(destination > u64::MAX - raw_amount);
            }
            Err(_) => unreachable!(),
        }
    }
}
