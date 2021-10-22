#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod interface;
pub use crate::interface::IPFS;

#[frame_support::pallet]
pub mod pallet {
	use crate::interface::IPFS;
	use codec::{Decode, Encode};
	use frame_support::{pallet_prelude::*, sp_runtime::traits::Hash};
	use frame_system::{
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
			SignedPayload, Signer, SigningTypes, SubmitTransaction,
		},
		pallet_prelude::*,
	};
	use log;
	use scale_info::TypeInfo;
	use sp_core::{
		crypto::KeyTypeId,
		offchain::{Duration, IpfsRequest, IpfsResponse, OpaqueMultiaddr, Timestamp},
	};
	use sp_io::offchain::timestamp;
	use sp_runtime::offchain::ipfs;
	use sp_std::{str, vec::Vec};

	/// Defines application identifier for crypto keys of this module.
	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ipfs");

	pub mod crypto {
		use crate::KEY_TYPE;
		use sp_core::ed25519::Signature as Ed25519Signature;

		use sp_runtime::{
			app_crypto::{app_crypto, ed25519},
			traits::Verify,
			MultiSignature, MultiSigner,
		};
		use sp_std::prelude::*;

		app_crypto!(ed25519, KEY_TYPE);

		pub struct AuthId;
		// implemented for ocw-runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for AuthId {
			type RuntimeAppPublic = Public;
			type GenericPublic = sp_core::ed25519::Public;
			type GenericSignature = sp_core::ed25519::Signature;
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
	pub struct Payload<Public> {
		number: u64,
		public: Public,
	}

	impl<T: SigningTypes> SignedPayload<T> for Payload<T::Public> {
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The overarching dispatch call type.
		type Call: From<Call<Self>>;
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	/// Defines file ID based on content hash.
	pub type FileId<T> = <T as frame_system::Config>::Hash;

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
		DataAdded(FileId<T>, Vec<u8>),
		DataRetrieved(FileId<T>, Vec<u8>),
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum ConnectionCommand {
		ConnectTo(OpaqueMultiaddr),
		DisconnectFrom(OpaqueMultiaddr),
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum DataCommand<T: Config> {
		AddBytes((FileId<T>, Vec<u8>)),
		CatBytes(Vec<u8>),
		InsertPin(Vec<u8>),
		RemoveBlock(Vec<u8>),
		RemovePin(Vec<u8>),
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub enum DhtCommand {
		FindPeer(Vec<u8>),
		GetProviders(Vec<u8>),
	}

	#[pallet::error]
	pub enum Error<T> {
		RequestCreateFailed,
		RequestTimeout,
		RequestFailed,
		OffchainAddBytesFailed,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	// A list of addresses to connect to and disconnect from.
	pub(super) type ConnectionQueue<T: Config> =
		StorageValue<_, Vec<ConnectionCommand>, ValueQuery>;

	#[pallet::storage]
	// A queue of data to publish or obtain on IPFS.
	pub(super) type DataQueue<T: Config> = StorageValue<_, Vec<DataCommand<T>>, ValueQuery>;

	#[pallet::storage]
	// A list of requests to the DHT.
	pub(super) type DhtQueue<T: Config> = StorageValue<_, Vec<DhtCommand>, ValueQuery>;

	#[pallet::storage]
	// Offchain job results associated with file hash.
	pub(super) type OutboundValues<T: Config> =
		StorageMap<_, Blake2_128Concat, FileId<T>, Vec<u8>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: T::BlockNumber) -> Weight {
			<ConnectionQueue<T>>::kill();
			<DhtQueue<T>>::kill();

			if block_number % 2u32.into() == 1u32.into() {
				<DhtQueue<T>>::kill();
			}

			0
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			// process connect/disconnect commands
			if let Err(e) = Self::connection_housekeeping() {
				log::debug!("IPFS: Encountered an error during connection housekeeping: {:?}", e);
			}

			// process requests to the DHT
			if let Err(e) = Self::handle_dht_requests() {
				log::debug!("IPFS: Encountered an error while processing DHT requests: {:?}", e);
			}

			// process Ipfs::{add, get} queues every other block
			if (block_number % 2u32.into()) == 1u32.into() {
				if let Err(e) = Self::handle_data_requests() {
					log::debug!(
						"IPFS: Encountered an error while processing data requests: {:?}",
						e
					);
				}
			}

			// display some stats every 5 blocks
			if (block_number % 5u32.into()) == 0u32.into() {
				if let Err(e) = Self::print_metadata() {
					log::error!("IPFS: Encountered an error while obtaining metadata: {:?}", e);
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Mark a `Multiaddr` as a desired connection target. The connection will be established
		/// during the next run of the off-chain `connection_housekeeping` process.
		#[pallet::weight(100_000)]
		pub fn connect(origin: OriginFor<T>, addr: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<Self as IPFS>::connect(addr)?;

			Self::deposit_event(Event::ConnectionRequested(who));

			Ok(())
		}

		/// Queues a `Multiaddr` to be disconnected. The connection will be severed during the next
		/// run of the off-chain `connection_housekeeping` process.
		#[pallet::weight(500_000)]
		pub fn disconnect(origin: OriginFor<T>, addr: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<Self as IPFS>::disconnect(addr)?;

			Self::deposit_event(Event::DisconnectRequested(who));

			Ok(())
		}

		/// Add arbitrary bytes to the IPFS repository. The registered `Cid` is printed out in the
		/// logs.
		#[pallet::weight(200_000)]
		pub fn add_bytes(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<Self as IPFS>::add_bytes(data)?;

			Self::deposit_event(Event::QueuedDataToAdd(who));

			Ok(())
		}

		/// Find IPFS data pointed to by the given `Cid`; if it is valid UTF-8, it is printed in the
		/// logs verbatim; otherwise, the decimal representation of the bytes is displayed instead.
		#[pallet::weight(100_000)]
		pub fn cat_bytes(origin: OriginFor<T>, cid: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<Self as IPFS>::cat_bytes(cid)?;

			Self::deposit_event(Event::QueuedDataToCat(who));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn submit_outbound_values(
			origin: OriginFor<T>,
			file_id: FileId<T>,
			values: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			<OutboundValues<T>>::insert(file_id, values.clone());

			Self::deposit_event(Event::DataAdded(file_id, values));

			Ok(())
		}
	}

	impl<T: Config> IPFS for Pallet<T> {
		fn connect(addr: Vec<u8>) -> DispatchResult {
			let cmd = ConnectionCommand::ConnectTo(OpaqueMultiaddr(addr));

			ConnectionQueue::<T>::mutate(|cmds| {
				if !cmds.contains(&cmd) {
					cmds.push(cmd)
				}
			});

			Ok(())
		}

		fn disconnect(addr: Vec<u8>) -> DispatchResult {
			let cmd = ConnectionCommand::DisconnectFrom(OpaqueMultiaddr(addr));

			ConnectionQueue::<T>::mutate(|cmds| {
				if !cmds.contains(&cmd) {
					cmds.push(cmd)
				}
			});

			Ok(())
		}

		fn add_bytes(data: Vec<u8>) -> DispatchResult {
			let file_id = T::Hashing::hash(&data);
			DataQueue::<T>::append(DataCommand::<T>::AddBytes((file_id, data)));

			Ok(())
		}

		fn cat_bytes(cid: Vec<u8>) -> DispatchResult {
			DataQueue::<T>::append(DataCommand::<T>::CatBytes(cid));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// send a request to the local IPFS node; can only be called be an off-chain worker
		fn ipfs_request(
			req: IpfsRequest,
			deadline: impl Into<Option<Timestamp>>,
		) -> Result<IpfsResponse, Error<T>> {
			let ipfs_request =
				ipfs::PendingRequest::new(req).map_err(|_| Error::<T>::RequestCreateFailed)?;
			ipfs_request
				.try_wait(deadline)
				.map_err(|_| Error::<T>::RequestTimeout)?
				.map(|r| r.response)
				.map_err(|e| {
					if let ipfs::Error::IoError(err) = e {
						log::debug!("IPFS: request failed: {}", str::from_utf8(&err).unwrap());
					} else {
						log::debug!("IPFS: request failed: {:?}", e);
					}
					Error::<T>::RequestFailed
				})
		}

		fn connection_housekeeping() -> Result<(), Error<T>> {
			let mut deadline;

			for cmd in ConnectionQueue::<T>::get() {
				deadline = Some(timestamp().add(Duration::from_millis(1_000)));

				match cmd {
					// connect to the desired peers if not yet connected
					ConnectionCommand::ConnectTo(addr) => {
						match Self::ipfs_request(IpfsRequest::Connect(addr.clone()), deadline) {
							Ok(IpfsResponse::Success) => {
								log::info!(
									"IPFS: connected to {}",
									str::from_utf8(&addr.0)
										.expect("our own calls can be trusted to be UTF-8; qed")
								);
							}
							Ok(_) => unreachable!(
								"only Success can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: connect error: {:?}", e),
						}
					}
					// disconnect from peers that are no longer desired
					ConnectionCommand::DisconnectFrom(addr) => {
						match Self::ipfs_request(IpfsRequest::Disconnect(addr.clone()), deadline) {
							Ok(IpfsResponse::Success) => {
								log::info!(
									"IPFS: disconnected from {}",
									str::from_utf8(&addr.0)
										.expect("our own calls can be trusted to be UTF-8; qed")
								);
							}
							Ok(_) => unreachable!(
								"only Success can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: disconnect error: {:?}", e),
						}
					}
				}
			}

			Ok(())
		}

		fn handle_dht_requests() -> Result<(), Error<T>> {
			let mut deadline;

			for cmd in DhtQueue::<T>::get() {
				deadline = Some(timestamp().add(Duration::from_millis(1_000)));

				match cmd {
					// find the known addresses of the given peer
					DhtCommand::FindPeer(peer_id) => {
						match Self::ipfs_request(IpfsRequest::FindPeer(peer_id.clone()), deadline) {
							Ok(IpfsResponse::FindPeer(addrs)) => {
								log::info!(
									"IPFS: found the following addresses of {}: {:?}",
									str::from_utf8(&peer_id)
										.expect("our own calls can be trusted to be UTF-8; qed"),
									addrs
										.iter()
										.map(|addr| str::from_utf8(&addr.0).expect(
											"our node's results can be trusted to be UTF-8; qed"
										))
										.collect::<Vec<_>>()
								);
							}
							Ok(_) => unreachable!(
								"only FindPeer can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: find peer error: {:?}", e),
						}
					}
					// disconnect from peers that are no longer desired
					DhtCommand::GetProviders(cid) => {
						match Self::ipfs_request(IpfsRequest::GetProviders(cid.clone()), deadline) {
							Ok(IpfsResponse::GetProviders(peer_ids)) => {
								log::info!(
									"IPFS: found the following providers of {}: {:?}",
									str::from_utf8(&cid)
										.expect("our own calls can be trusted to be UTF-8; qed"),
									peer_ids
										.iter()
										.map(|peer_id| str::from_utf8(&peer_id).expect(
											"our node's results can be trusted to be UTF-8; qed"
										))
										.collect::<Vec<_>>()
								);
							}
							Ok(_) => unreachable!(
								"only GetProviders can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: find providers error: {:?}", e),
						}
					}
				}
			}

			Ok(())
		}

		fn handle_data_requests() -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();
			let data_queue = DataQueue::<T>::get();
			let len = data_queue.len();
			if len != 0 {
				log::info!(
					"IPFS: {} entr{} in the data queue",
					len,
					if len == 1 { "y" } else { "ies" }
				);
			}

			let deadline = Some(timestamp().add(Duration::from_millis(1_000)));
			for cmd in data_queue.into_iter() {
				match cmd {
					DataCommand::<T>::AddBytes((file_id, data)) => {
						match Self::ipfs_request(IpfsRequest::AddBytes(data.clone()), deadline) {
							Ok(IpfsResponse::AddBytes(cid)) => {
								log::info!(
									"IPFS: added data with Cid {}",
									str::from_utf8(&cid)
										.expect("our own IPFS node can be trusted here; qed")
								);

								if let Some((acc, res)) = signer.send_signed_transaction(|_acct| {
									Call::submit_outbound_values { file_id, values: cid.clone() }
								}) {
									if res.is_err() {
										log::error!("failed send results: {:?}", acc.id);
										return Err(<Error<T>>::OffchainAddBytesFailed);
									}

									return Ok(());
								}
							}
							Ok(_) => unreachable!(
								"only AddBytes can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: add error: {:?}", e),
						}
					}
					DataCommand::<T>::CatBytes(data) => {
						match Self::ipfs_request(IpfsRequest::CatBytes(data.clone()), deadline) {
							Ok(IpfsResponse::CatBytes(data)) => {
								if let Ok(str) = str::from_utf8(&data) {
									log::info!("IPFS: got data: {:?}", str);
								} else {
									log::info!("IPFS: got data: {:x?}", data);
								};
							}
							Ok(_) => unreachable!(
								"only CatBytes can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: error: {:?}", e),
						}
					}
					DataCommand::<T>::RemoveBlock(cid) => {
						match Self::ipfs_request(IpfsRequest::RemoveBlock(cid), deadline) {
							Ok(IpfsResponse::RemoveBlock(cid)) => {
								log::info!(
									"IPFS: removed a block with Cid {}",
									str::from_utf8(&cid)
										.expect("our own IPFS node can be trusted here; qed")
								);
							}
							Ok(_) => unreachable!(
								"only RemoveBlock can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: remove block error: {:?}", e),
						}
					}
					DataCommand::<T>::InsertPin(cid) => {
						match Self::ipfs_request(
							IpfsRequest::InsertPin(cid.clone(), false),
							deadline,
						) {
							Ok(IpfsResponse::Success) => {
								log::info!(
									"IPFS: pinned data with Cid {}",
									str::from_utf8(&cid)
										.expect("our own request can be trusted to be UTF-8; qed")
								);
							}
							Ok(_) => unreachable!(
								"only Success can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: insert pin error: {:?}", e),
						}
					}
					DataCommand::<T>::RemovePin(cid) => {
						match Self::ipfs_request(
							IpfsRequest::RemovePin(cid.clone(), false),
							deadline,
						) {
							Ok(IpfsResponse::Success) => {
								log::info!(
									"IPFS: unpinned data with Cid {}",
									str::from_utf8(&cid)
										.expect("our own request can be trusted to be UTF-8; qed")
								);
							}
							Ok(_) => unreachable!(
								"only Success can be a response for that request type; qed"
							),
							Err(e) => log::debug!("IPFS: remove pin error: {:?}", e),
						}
					}
				}
			}

			Ok(())
		}

		fn print_metadata() -> Result<(), Error<T>> {
			let deadline = Some(timestamp().add(Duration::from_millis(200)));

			let peers = if let IpfsResponse::Peers(peers) =
				Self::ipfs_request(IpfsRequest::Peers, deadline)?
			{
				peers
			} else {
				unreachable!("only Peers can be a response for that request type; qed");
			};
			let peer_count = peers.len();

			log::info!(
				"IPFS: currently connected to {} peer{}",
				peer_count,
				if peer_count == 1 { "" } else { "s" },
			);

			Ok(())
		}
	}
}
