use crate::pallet::Config as PalletConfig;
use frame_support::{pallet_prelude::*, traits::Currency};

use scale_info::TypeInfo;
use sp_runtime::Perbill;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as PalletConfig>::Currency as Currency<AccountIdOf<T>>>::Balance;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationParameters {
	pub inflation_rate: Perbill,
	pub disinflation_rate: Perbill,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationConfiguration {
	pub inflation_parameters: InflationParameters,
	pub initial_disinflation: Perbill,
	pub inflation_stagnation_rate: Perbill,
	pub inflation_stagnation_year: u128,
}
