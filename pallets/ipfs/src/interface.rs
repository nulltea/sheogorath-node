use frame_support::{
	dispatch::{result::Result, DispatchError, DispatchResult},
	traits::Get,
};
use frame_system::pallet_prelude::*;
use sp_std::{vec::Vec};

pub trait IPFS {
	fn connect(addr: Vec<u8>) -> DispatchResult;

	fn disconnect(addr: Vec<u8>) -> DispatchResult;

	fn add_bytes(data: Vec<u8>) -> DispatchResult;

	fn cat_bytes(cid: Vec<u8>) -> DispatchResult;
}

