#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod interface;

#[frame_support::pallet]
pub mod pallet {
	use crate::interface::UniqueAssets;

	use codec::FullCodec;
	use frame_support::{
		dispatch::{DispatchResult},
		pallet_prelude::*,
		sp_runtime::traits::Hash,
		traits::{Currency},
		Hashable,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_std::{fmt::Debug, str, vec::Vec, collections::btree_map::BTreeMap};
	use pallet_ipfs::IPFS;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The dispatch origin that is able to mint new instances of this type of commodity.
		type AssetsCurator: EnsureOrigin<Self::Origin>;
		/// The data type that is used to describe this type of commodity.
		type AssetMetadata: Clone + Hashable + Debug + PartialEq + TypeInfo + FullCodec;
		/// The maximum number of this type of commodity that may exist (minted - burned).
		type AssetLimit: Get<u128>;
		/// The maximum number of this type of commodity that any single account may own.
		type UserAssetLimit: Get<u64>;
		/// ...
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The Currency handler for the Kitties pallet.
		type Currency: Currency<Self::AccountId>;
		/// IPFS client for persisting content data.
		type FileSystem: IPFS;
	}

	/// AssetId being its hash.
	pub type AssetId<T> = <T as frame_system::Config>::Hash;

	/// Associates a commodity with its ID.
	pub type Asset<T> = (AssetId<T>, <T as Config>::AssetMetadata);

	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct AssetMetadata<T: Config> {
		pub name: Vec<u8>,
		pub content_uri: Vec<u8>,
		pub properties: Option<BTreeMap<Vec<u8>, Vec<u8>>>,
		pub initial_price: Option<BalanceOf<T>>,
	}

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
		AssetAlreadyExists,
		AssetNotOwnedByAccount,
		AssetForAccountLimitExcited,
		AssetLimitExcited,
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
	pub(super) type TotalByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	/// A mapping from an account to a list of all of the commodities of this type that are owned by it.
	#[pallet::storage]
	#[pallet::getter(fn assets_by_account)]
	pub(super) type AssetsByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Asset<T>>, ValueQuery>;

	/// A mapping from a commodity ID to the account that owns it.
	#[pallet::storage]
	#[pallet::getter(fn account_by_asset_id)]
	pub(super) type AccountByAsset<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetId<T>, T::AccountId, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn mint(
			origin: OriginFor<T>,
			owner_account: T::AccountId,
			metadata: T::AssetMetadata,
		) -> DispatchResult {
			//T::AssetsCurator::ensure_origin(origin)?;
			ensure_signed(origin)?;

			let id = <Self as UniqueAssets<T::AccountId>>::mint(&owner_account, metadata)?;

			Self::deposit_event(Event::Minted(id, owner_account));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn burn(origin: OriginFor<T>, asset_id: AssetId<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				sender == Self::account_by_asset_id(&asset_id),
				<Error<T>>::AssetNotOwnedByAccount
			);

			<Self as UniqueAssets<T::AccountId>>::burn(&asset_id)?;

			Self::deposit_event(Event::Burned(asset_id));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			dest_account: T::AccountId,
			asset_id: AssetId<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				sender == Self::account_by_asset_id(&asset_id),
				<Error<T>>::AssetNotOwnedByAccount
			);

			<Self as UniqueAssets<T::AccountId>>::transfer(&dest_account, &asset_id)?;

			Self::deposit_event(Event::Transferred(asset_id.clone(), dest_account.clone()));

			Ok(())
		}

		// #[pallet::weight(10_000)]
		// pub fn mint(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
		// 	let sender = ensure_signed(origin)?;
		//
		// 	Ok(())
		// }
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

		fn total_for_account(account: &T::AccountId) -> u64 {
			Self::total_by_account(account)
		}

		fn assets_for_account(account: &T::AccountId) -> Vec<(Self::AssetId, Self::AssetInfo)> {
			Self::assets_by_account(account)
		}

		fn owner_of(asset_id: &Self::AssetId) -> T::AccountId {
			Self::account_by_asset_id(asset_id)
		}

		fn mint(
			owner_account: &T::AccountId,
			asset_info: Self::AssetInfo,
		) -> Result<Self::AssetId, DispatchError> {
			let asset_id = T::Hashing::hash_of(&asset_info);

			ensure!(!AccountByAsset::<T>::contains_key(&asset_id), <Error<T>>::AssetAlreadyExists);

			ensure!(
				T::UserAssetLimit::get() > Self::total_by_account(owner_account),
				<Error<T>>::AssetForAccountLimitExcited
			);

			ensure!(T::AssetLimit::get() > Self::total(), Error::<T>::AssetLimitExcited);

			let asset = (asset_id, asset_info) as Asset<T>;

			<Total<T>>::mutate(|t| *t += 1);
			<TotalByAccount<T>>::mutate(&owner_account, |t| *t += 1);
			<AssetsByAccount<T>>::append(&owner_account, asset);
			<AccountByAsset<T>>::insert(&asset_id, owner_account);

			Ok(asset_id)
		}

		fn burn(asset_id: &Self::AssetId) -> DispatchResult {
			let owner_account = Self::owner_of(asset_id);
			ensure!(owner_account != T::AccountId::default(), <Error<T>>::AssetNotOwnedByAccount);

			<Total<T>>::mutate(|t| *t -= 1);
			<Burned<T>>::mutate(|b| *b += 1);
			<TotalByAccount<T>>::mutate(&owner_account, |t| *t -= 1);
			<AssetsByAccount<T>>::mutate(&owner_account, |assets| {
				assets.remove(
					assets
						.binary_search_by_key(asset_id, |(x, _)| *x)
						.expect("record expected to be in vector"),
				);
			});
			<AccountByAsset<T>>::remove(asset_id);

			Ok(())
		}

		fn transfer(dest_account: &T::AccountId, asset_id: &Self::AssetId) -> DispatchResult {
			let owner_account = Self::owner_of(asset_id);

			ensure!(owner_account != T::AccountId::default(), <Error<T>>::AssetNotOwnedByAccount);

			ensure!(
				T::UserAssetLimit::get() > Self::total_by_account(dest_account),
				<Error<T>>::AssetForAccountLimitExcited
			);

			<TotalByAccount<T>>::mutate(&owner_account, |t| *t -= 1);
			<TotalByAccount<T>>::mutate(dest_account, |t| *t += 1);

			let asset = AssetsByAccount::<T>::mutate(&owner_account, |assets| {
				assets.remove(
					assets
						.binary_search_by_key(&asset_id, |(x, _)| x)
						.expect("record expected to be in vector"),
				)
			});

			<AssetsByAccount<T>>::append(dest_account, asset);
			<AccountByAsset<T>>::insert(asset_id, dest_account);

			Ok(())
		}
	}
}
