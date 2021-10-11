#![cfg_attr(not(feature = "std"), no_std)]

pub use nft::*;

#[frame_support::pallet]
pub mod nft {
	use frame_support::{pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	//use sp_std::vec::Vec;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a Account balance changed. [who, value]
		BalanceChanged(T::AccountId, f64),
	}

	#[pallet::error]
	pub enum Error<T> {

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type Collection<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, f64, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn credit_value(origin: OriginFor<T>, value: f64) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let new_value = match Collection::<T>::try_get(&sender) {
				Ok(current) => current + value,
				Err(_) => value,
			};

			Collection::<T>::insert(&sender, new_value);

			Self::deposit_event(Event::BalanceChanged(sender, new_value));

			Ok(())
		}

		// #[pallet::weight(10_000)]
		// pub fn mint(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
		// 	let sender = ensure_signed(origin)?;
		//
		// 	Ok(())
		// }
	}
}
