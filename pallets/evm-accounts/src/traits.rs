use peaq_primitives_xcm::{evm::EvmAddress};

/// A mapping between `AccountId` and `EvmAddress`.
pub trait EVMAddressMapping<AccountId> {
	/// Returns the AccountId used go generate the given EvmAddress.
	fn get_account_id(evm: &EvmAddress) -> AccountId;
	/// Returns the EvmAddress associated with a given AccountId or the
	/// underlying EvmAddress of the AccountId.
	/// Returns None if there is no EvmAddress associated with the AccountId
	/// and there is no underlying EvmAddress in the AccountId.
	fn get_evm_address(account_id: &AccountId) -> Option<EvmAddress>;
	/// Returns the EVM address associated with an account ID and generates an
	/// account mapping if no association exists.
	fn get_or_create_evm_address(account_id: &AccountId) -> EvmAddress;
	/// Returns the default EVM address associated with an account ID.
	fn get_default_evm_address(account_id: &AccountId) -> EvmAddress;
	/// Returns true if a given AccountId is associated with a given EvmAddress
	/// and false if is not.
	fn is_linked(account_id: &AccountId, evm: &EvmAddress) -> bool;
}
