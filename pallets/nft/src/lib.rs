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

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The dispatch origin that is able to mint new instances of this type of commodity.
		type Curator: EnsureOrigin<Self::Origin>;
		/// The data type that is used to describe this type of commodity.
		type Metadata: Hashable + Member + Debug + Default + FullCodec + Ord;
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
		/// Event emitted when a Account value changed. [who, value]
		ValueChanged(T::AccountId, u64),
	}

	#[pallet::error]
	pub enum Error<T> {

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	/// The total number of this type of commodity that exists (minted - burned).
	pub(super) type Total<T: Config> = StorageValue<_, u128, ValueQuery>;
	/// The total number of this type of commodity that has been burned
	pub(super) type Burned<T: Config> = StorageValue<_, u128, ValueQuery>;
	/// The total number of this type of commodity owned by an account.
	pub(super) type TotalForAccount<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;
	/// A mapping from an account to a list of all of the commodities of this type that are owned by it.
	pub(super) type AssetForAccount<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Asset<T, I>>, ValueQuery>;
	/// A mapping from a commodity ID to the account that owns it.
	pub(super) type AccountForAsset<T: Config> = StorageMap<_, Blake2_128Concat, AssetId<T>, T::AccountId, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn credit_value(origin: OriginFor<T>, value: u64) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let new_value = match Collection::<T>::try_get(&sender) {
				Ok(current) => current + value,
				Err(_) => value,
			};

			Collection::<T>::insert(&sender, new_value);

			Self::deposit_event(Event::ValueChanged(sender, new_value));

			Ok(())
		}
	}
}
