//! Source: https://github.com/danforbes/pallet-nft/blob/master/src/nft.rs

use super::*;

use frame_support::{
	dispatch::{result::Result, DispatchError, DispatchResult},
	traits::Get,
};
use sp_std::vec::Vec;

/// An interface over a set of unique assets.
/// Assets with equivalent attributes (as defined by the AssetInfo type) **must** have an equal ID
/// and assets with different IDs **must not** have equivalent attributes.
pub trait UniqueAssets<AccountId> {
	/// The type used to identify unique assets.
	type AssetID;
	/// The attributes that distinguish unique assets.
	type AssetInfo;
	/// The maximum number of this type of asset that may exist (minted - burned).
	type AssetLimit: Get<u128>;
	/// The maximum number of this type of asset that any single account may own.
	type UserAssetLimit: Get<u64>;

	/// The total number of this type of asset that exists (minted - burned).
	fn total() -> u128;
	/// The total number of this type of asset that has been burned (may overflow).
	fn burned() -> u128;
	/// The total number of this type of asset owned by an account.
	fn total_for_account(account: &AccountId) -> u64;
	/// The set of unique assets owned by an account.
	fn assets_for_account(account: &AccountId) -> Vec<(Self::AssetID, Self::AssetInfo)>;
	/// The ID of the account that owns an asset.
	fn owner_of(asset_id: &Self::AssetID) -> AccountId;

	/// Use the provided asset info to create a new unique asset for the specified user.
	fn mint(
		owner_account: &AccountId,
		asset_info: Self::AssetInfo,
	) -> Result<Self::AssetID, DispatchError>;
	/// Destroy an asset.
	fn burn(asset_id: &Self::AssetID) -> DispatchResult;
	/// Transfer ownership of an asset to another account.
	fn transfer(dest_account: &AccountId, asset_id: &Self::AssetID) -> DispatchResult;
}
