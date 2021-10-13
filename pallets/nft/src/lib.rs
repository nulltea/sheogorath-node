#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod nft;

#[frame_support::pallet]
pub mod pallet {
	use codec::FullCodec;
	use frame_support::{
		sp_runtime::traits::{Hash, Zero},
		dispatch::{DispatchResultWithPostInfo, DispatchResult},
		traits::{Currency, ExistenceRequirement, Randomness},
		Hashable,
		pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{str, vec::Vec, fmt::Debug};
	use crate::nft::UniqueAssets;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The dispatch origin that is able to mint new instances of this type of commodity.
		type AssetsCurator: EnsureOrigin<Self::Origin>;
		/// The data type that is used to describe this type of commodity.
		type AssetMetadata: Hashable + Member + Debug + Default + FullCodec + Ord;
		/// The maximum number of this type of commodity that may exist (minted - burned).
		type AssetLimit: Get<u128>;
		/// The maximum number of this type of commodity that any single account may own.
		type UserAssetLimit: Get<u64>;
	}

	/// The runtime system's hashing algorithm is used to uniquely identify commodities.
	pub type AssetId<T> = <T as frame_system::Config>::Hash;

	/// Associates a commodity with its ID.
	pub type Asset<T, I> = (AssetId<T>, <T as Trait<I>>::CommodityInfo);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The asset has been burned.
		Burned(AssetId<T>),
		/// The asset has been minted and distributed to the account.
		Minted(AssetId<T>, T::AccountId),
		/// Ownership of the asset has been transferred to the account.
		Transferred(AssetId<T>, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The total number of this type of commodity that exists (minted - burned).
	#[pallet::storage]
	#[pallet::getter(fn total)]
	pub(super) type Total<T: Config> = StorageValue<_, u128, ValueQuery>;
	/// The total number of this type of commodity that has been burned.
	#[pallet::storage]
	#[pallet::getter(fn burned)]
	pub(super) type Burned<T: Config> = StorageValue<_, u128, ValueQuery>;
	/// The total number of this type of commodity owned by an account.
	#[pallet::storage]
	#[pallet::getter(fn total_by_account)]
	pub(super) type TotalByAccount<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;
	/// A mapping from an account to a list of all of the commodities of this type that are owned by it.
	#[pallet::storage]
	#[pallet::getter(fn assets_by_account)]
	pub(super) type AssetsByAccount<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Asset<T, I>>, ValueQuery>;
	/// A mapping from a commodity ID to the account that owns it.
	#[pallet::storage]
	#[pallet::getter(fn account_by_asset_id)]
	pub(super) type AccountByAsset<T: Config> = StorageMap<_, Blake2_128Concat, AssetId<T>, T::AccountId, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn mint(origin: OriginFor<T>, owner_account: T::AccountId, metadata: T::AssetMetadata) -> DispatchResult {
			T::AssetsCurator::ensure_origin(origin)?;

			let id = <Self as UniqueAssets<T::AccountId>>::mint(&owner_account, metadata)?;

			Self::deposit_event(Event::Minted(id, owner_account));

			Ok(())
		}
	}

	impl<T: Config> UniqueAssets<T::AccountId> for Pallet<T> {
		type AssetId = AssetId<T>;
		type AssetInfo = T::AssetMetadata;
		type AssetLimit = T::AssetLimit;
		type UserAssetLimit = T::UserAssetLimit;

		fn total() -> u128 {
			Self::total()
		}

		fn burned() -> u128 {
			Self::burned()
		}

		fn total_for_account(account: &frame_system::pallet::AccountId) -> u64 {
			Self::total_by_account(account)
		}

		fn assets_for_account(account: &frame_system::pallet::AccountId) -> Vec<(Self::AssetID, Self::AssetInfo)> {
			Self::total_by_account(account)
		}

		fn owner_of(asset_id: &Self::AssetID) -> frame_system::pallet::AccountId {
			Self::account_by_asset_id(asset_id)
		}

		fn mint(owner_account: &frame_system::pallet::AccountId, asset_info: Self::AssetInfo) -> Result<Self::AssetID, DispatchError> {
			todo!()
		}

		fn burn(asset_id: &Self::AssetID) -> DispatchResult {
			todo!()
		}

		fn transfer(dest_account: &frame_system::pallet::AccountId, asset_id: &Self::AssetID) -> DispatchResult {
			todo!()
		}
	}
}
