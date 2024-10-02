// EoT Blockchain
// Copyright (C) 2019-2023 EoTLabs GmbH

// The EoTLabs Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The EoTLabs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Unit testing

use crate::{
	mock::{
		roll_to, Balances, BlockNumber, ExtBuilder, PCall, Precompiles, PrecompilesValue,
		RuntimeOrigin, StakePallet, Test,
	},
	Address, BalanceOf, CollatorInfo, U256,
};
use frame_support::{
	assert_ok, storage::bounded_btree_map::BoundedBTreeMap, traits::LockIdentifier,
};
use pallet_balances::{BalanceLock, Reasons};
use parachain_staking::types::TotalStake;
use precompile_utils::testing::{MockPeaqAccount, PrecompileTesterExt, PrecompilesModifierTester};
use sp_core::H256;

const STAKING_ID: LockIdentifier = *b"peaqstak";

fn precompiles() -> Precompiles<Test> {
	PrecompilesValue::get()
}

/// In the precompile the account is converted to a H256
/// But because the accountID32 cannot convert to H256,
/// We have to convert it to [u8; 32] first
/// Then convert it to H256
fn convert_mock_account_by_u8_list(account: MockPeaqAccount) -> H256 {
	H256::from(<[u8; 32]>::from(account))
}

#[test]
fn test_selector_enum() {
	assert!(PCall::get_collator_list_selectors().contains(&0xaaacb283));
	assert!(PCall::join_delegators_selectors().contains(&0xd9f511cd));
	assert!(PCall::delegate_another_candidate_selectors().contains(&0x1916fdca));
	assert!(PCall::leave_delegators_selectors().contains(&0x4b99dc38));
	assert!(PCall::revoke_delegation_selectors().contains(&0xb96f2b07));
	assert!(PCall::delegator_stake_more_selectors().contains(&0x1b3d3cdf));
	assert!(PCall::delegator_stake_less_selectors().contains(&0xb7e8947f));
	assert!(PCall::unlock_unstaked_selectors().contains(&0x0f615369));
}

#[test]
fn modifiers() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 10)])
		.with_collators(vec![(MockPeaqAccount::Alice, 10)])
		.build()
		.execute_with(|| {
			let mut tester = PrecompilesModifierTester::new(
				precompiles(),
				MockPeaqAccount::Alice,
				MockPeaqAccount::EVMu1Account,
			);

			tester.test_view_modifier(PCall::get_collator_list_selectors());
		});
}

#[test]
fn collator_list_test() {
	// same_unstaked_as_restaked
	// block 1: stake & unstake for 100
	// block 2: stake & unstake for 100
	// should remove first entry in unstaking BoundedBTreeMap when staking in block
	// 2 should still have 100 locked until unlocking
	ExtBuilder::default()
		.with_balances(vec![
			(MockPeaqAccount::Alice, 10),
			(MockPeaqAccount::Bob, 100),
			(MockPeaqAccount::Charlie, 100),
		])
		.with_collators(vec![(MockPeaqAccount::Alice, 10), (MockPeaqAccount::Charlie, 20)])
		.with_delegators(vec![(MockPeaqAccount::Bob, MockPeaqAccount::Alice, 100)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::get_collator_list {},
				)
				.expect_no_logs()
				.execute_returns(vec![
					CollatorInfo {
						owner: convert_mock_account_by_u8_list(MockPeaqAccount::Alice),
						amount: U256::from(110),
					},
					CollatorInfo {
						owner: convert_mock_account_by_u8_list(MockPeaqAccount::Charlie),
						amount: U256::from(20),
					},
				]);
		});
}

#[test]
fn unlock_unstaked() {
	// same_unstaked_as_restaked
	// block 1: stake & unstake for 100
	// block 2: stake & unstake for 100
	// should remove first entry in unstaking BoundedBTreeMap when staking in block
	// 2 should still have 100 locked until unlocking
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 10), (MockPeaqAccount::Bob, 100)])
		.with_collators(vec![(MockPeaqAccount::Alice, 10)])
		.with_delegators(vec![(MockPeaqAccount::Bob, MockPeaqAccount::Alice, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(StakePallet::revoke_delegation(
				RuntimeOrigin::signed(MockPeaqAccount::Bob),
				MockPeaqAccount::Alice
			));
			let mut unstaking: BoundedBTreeMap<
				BlockNumber,
				BalanceOf<Test>,
				<Test as parachain_staking::Config>::MaxUnstakeRequests,
			> = BoundedBTreeMap::new();
			assert_ok!(unstaking.try_insert(3, 100));
			let lock = BalanceLock { id: STAKING_ID, amount: 100, reasons: Reasons::All };
			assert_eq!(StakePallet::unstaking(MockPeaqAccount::Bob), unstaking);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![lock.clone()]);
			// shouldn't be able to unlock anything
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::unlock_unstaked { target: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_no_logs()
				.execute_returns(());
			assert_eq!(StakePallet::unstaking(MockPeaqAccount::Bob), unstaking);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![lock.clone()]);

			// join delegators and revoke again --> consume unstaking at block 3
			roll_to(2, vec![]);
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::join_delegators {
						collator: convert_mock_account_by_u8_list(MockPeaqAccount::Alice),
						stake: 100.into(),
					},
				)
				.expect_no_logs()
				.execute_returns(());

			assert_ok!(StakePallet::revoke_delegation(
				RuntimeOrigin::signed(MockPeaqAccount::Bob),
				MockPeaqAccount::Alice
			));
			unstaking.remove(&3);
			assert_ok!(unstaking.try_insert(4, 100));
			assert_eq!(StakePallet::unstaking(MockPeaqAccount::Bob), unstaking);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![lock.clone()]);
			// shouldn't be able to unlock anything
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::unlock_unstaked { target: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_no_logs()
				.execute_returns(());
			assert_eq!(StakePallet::unstaking(MockPeaqAccount::Bob), unstaking);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![lock.clone()]);

			// should reduce unlocking but not unlock anything
			roll_to(3, vec![]);
			assert_eq!(StakePallet::unstaking(MockPeaqAccount::Bob), unstaking);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![lock.clone()]);
			// shouldn't be able to unlock anything
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::unlock_unstaked { target: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_no_logs()
				.execute_returns(());
			assert_eq!(StakePallet::unstaking(MockPeaqAccount::Bob), unstaking);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![lock.clone()]);

			roll_to(4, vec![]);
			unstaking.remove(&4);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![lock]);
			// shouldn't be able to unlock anything
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::unlock_unstaked { target: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_no_logs()
				.execute_returns(());
			assert_eq!(StakePallet::unstaking(MockPeaqAccount::Bob), unstaking);
			assert_eq!(Balances::locks(MockPeaqAccount::Bob), vec![]);
		});
}

#[test]
fn should_update_total_stake() {
	ExtBuilder::default()
		.with_balances(vec![
			(MockPeaqAccount::Alice, 100),
			(MockPeaqAccount::Bob, 100),
			(MockPeaqAccount::Charlie, 100),
			(MockPeaqAccount::David, 500),
			(MockPeaqAccount::ParentAccount, 100),
		])
		.with_collators(vec![(MockPeaqAccount::Alice, 30), (MockPeaqAccount::ParentAccount, 30)])
		.with_delegators(vec![
			(MockPeaqAccount::Bob, MockPeaqAccount::Alice, 20),
			(MockPeaqAccount::Charlie, MockPeaqAccount::Alice, 20),
		])
		.set_blocks_per_round(5)
		.build()
		.execute_with(|| {
			let mut old_stake = StakePallet::total_collator_stake();
			assert_eq!(old_stake, TotalStake { collators: 60, delegators: 40 });

			old_stake = StakePallet::total_collator_stake();
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::delegator_stake_more {
						collator: convert_mock_account_by_u8_list(MockPeaqAccount::Alice),
						stake: 50.into(),
					},
				)
				.expect_no_logs()
				.execute_returns(());

			assert_eq!(
				StakePallet::total_collator_stake(),
				TotalStake { delegators: old_stake.delegators + 50, ..old_stake }
			);

			old_stake = StakePallet::total_collator_stake();
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::delegator_stake_less {
						collator: convert_mock_account_by_u8_list(MockPeaqAccount::Alice),
						stake: 50.into(),
					},
				)
				.expect_no_logs()
				.execute_returns(());
			assert_eq!(
				StakePallet::total_collator_stake(),
				TotalStake { delegators: old_stake.delegators - 50, ..old_stake }
			);

			old_stake = StakePallet::total_collator_stake();
			precompiles()
				.prepare_test(
					MockPeaqAccount::David,
					MockPeaqAccount::EVMu1Account,
					PCall::join_delegators {
						collator: convert_mock_account_by_u8_list(MockPeaqAccount::Alice),
						stake: 50.into(),
					},
				)
				.expect_no_logs()
				.execute_returns(());

			assert_eq!(
				StakePallet::total_collator_stake(),
				TotalStake { delegators: old_stake.delegators + 50, ..old_stake }
			);

			old_stake = StakePallet::total_collator_stake();
			precompiles()
				.prepare_test(
					MockPeaqAccount::David,
					MockPeaqAccount::EVMu1Account,
					PCall::delegate_another_candidate {
						collator: convert_mock_account_by_u8_list(MockPeaqAccount::ParentAccount),
						stake: 60.into(),
					},
				)
				.expect_no_logs()
				.execute_returns(());

			assert_eq!(
				StakePallet::total_collator_stake(),
				TotalStake { delegators: old_stake.delegators + 60, ..old_stake }
			);

			old_stake = StakePallet::total_collator_stake();
			assert_eq!(StakePallet::delegator_state(MockPeaqAccount::Charlie).unwrap().total, 20);
			precompiles()
				.prepare_test(
					MockPeaqAccount::Charlie,
					MockPeaqAccount::EVMu1Account,
					PCall::leave_delegators {},
				)
				.expect_no_logs()
				.execute_returns(());
			assert_eq!(
				StakePallet::total_collator_stake(),
				TotalStake { delegators: old_stake.delegators - 20, ..old_stake }
			);
			let old_stake = StakePallet::total_collator_stake();
			assert_eq!(StakePallet::delegator_state(MockPeaqAccount::Bob).unwrap().total, 20);
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::EVMu1Account,
					PCall::revoke_delegation {
						collator: convert_mock_account_by_u8_list(MockPeaqAccount::Alice),
					},
				)
				.expect_no_logs()
				.execute_returns(());

			assert_eq!(
				StakePallet::total_collator_stake(),
				TotalStake { delegators: old_stake.delegators - 20, ..old_stake }
			);
		})
}
