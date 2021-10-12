#![cfg_attr(not(feature = "std"), no_std)]

pub use ipfs::*;

#[frame_support::pallet]
pub mod ipfs {
	use codec::{Encode, Decode};
	use frame_support::{pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use sp_core::offchain::{Duration, IpfsRequest, IpfsResponse, OpaqueMultiaddr, Timestamp};
	use sp_runtime::offchain::ipfs;
	use sp_std::{str, vec::Vec};
	use scale_info::TypeInfo;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ConnectionRequested(T::AccountId),
		DisconnectRequested(T::AccountId),
		QueuedDataToAdd(T::AccountId),
		QueuedDataToCat(T::AccountId),
		QueuedDataToPin(T::AccountId),
		QueuedDataToRemove(T::AccountId),
		QueuedDataToUnpin(T::AccountId),
		FindPeerIssued(T::AccountId),
		FindProvidersIssued(T::AccountId),
	}

	#[derive(Encode, Decode, TypeInfo, PartialEq)]
	pub enum ConnectionCommand {
		ConnectTo(OpaqueMultiaddr),
		DisconnectFrom(OpaqueMultiaddr),
	}

	#[derive(Encode, Decode, TypeInfo, PartialEq)]
	pub enum DataCommand {
		AddBytes(Vec<u8>),
		CatBytes(Vec<u8>),
		InsertPin(Vec<u8>),
		RemoveBlock(Vec<u8>),
		RemovePin(Vec<u8>),
	}

	#[derive(Encode, Decode, TypeInfo, PartialEq)]
	pub enum DhtCommand {
		FindPeer(Vec<u8>),
		GetProviders(Vec<u8>),
	}

	#[pallet::error]
	pub enum Error<T> {
		CantCreateRequest,
		RequestTimeout,
		RequestFailed,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	// A list of addresses to connect to and disconnect from.
	pub(super) type ConnectionQueue<T: Config> = StorageValue<_, Vec<ConnectionCommand>, ValueQuery>;

	#[pallet::storage]
	// A queue of data to publish or obtain on IPFS.
	pub(super) type DataQueue<T: Config> = StorageValue<_, Vec<DataCommand>, ValueQuery>;

	#[pallet::storage]
	// A list of requests to the DHT.
	pub(super) type DhtQueue<T: Config> = StorageValue<_, Vec<DhtCommand>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Mark a `Multiaddr` as a desired connection target. The connection will be established
		/// during the next run of the off-chain `connection_housekeeping` process.
		#[pallet::weight(100_000)]
		pub fn ipfs_connect(origin: OriginFor<T>, addr: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let cmd = ConnectionCommand::ConnectTo(OpaqueMultiaddr(addr));

			ConnectionQueue::<T>::mutate(|cmds| if !cmds.contains(&cmd) { cmds.push(cmd) });
			Self::deposit_event(Event::ConnectionRequested(who));

			Ok(())
		}

		/// Queues a `Multiaddr` to be disconnected. The connection will be severed during the next
		/// run of the off-chain `connection_housekeeping` process.
		#[pallet::weight(500_000)]
		pub fn ipfs_disconnect(origin: OriginFor<T>, addr: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let cmd = ConnectionCommand::DisconnectFrom(OpaqueMultiaddr(addr));

			ConnectionQueue::<T>::mutate(|cmds| if !cmds.contains(&cmd) { cmds.push(cmd) });
			Self::deposit_event(Event::DisconnectRequested(who));


			Ok(())
		}

		/// Add arbitrary bytes to the IPFS repository. The registered `Cid` is printed out in the
		/// logs.
		#[pallet::weight(200_000)]
		pub fn ipfs_add_bytes(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			DataQueue::<T>::append(DataCommand::AddBytes(data));
			Self::deposit_event(Event::QueuedDataToAdd(who));


			Ok(())
		}

		/// Find IPFS data pointed to by the given `Cid`; if it is valid UTF-8, it is printed in the
		/// logs verbatim; otherwise, the decimal representation of the bytes is displayed instead.
		#[pallet::weight(100_000)]
		pub fn ipfs_cat_bytes(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			DataQueue::<T>::append(DataCommand::CatBytes(cid));
			Self::deposit_event(Event::QueuedDataToCat(who));


			Ok(())
		}
	}

}
