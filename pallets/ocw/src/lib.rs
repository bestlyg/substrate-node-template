#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;
use sp_std::vec::Vec;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	const ONCHAIN_TX_KEY: &[u8] = b"ocw::onchain::tx::key";
	#[derive(Debug, Encode, Decode, Default)]
	struct IndexingData(BoundedVec<u8, ConstU32<4>>);

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Something<T> = StorageValue<_, u32>;
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored { something: u32, who: T::AccountId },
		SetValue { who: T::AccountId, value: BoundedVec<u8, ConstU32<4>> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NoneValue,
		StorageOverflow,
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn set_value(
			origin: OriginFor<T>,
			value: BoundedVec<u8, ConstU32<4>>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			log::info!("set_value ==> set_value: {:?}", value);
			let data = IndexingData(value.clone());
			log::info!("set_value ==> set key: {:?}", ONCHAIN_TX_KEY);
			log::info!("set_value ==> set value: {:?}", sp_std::str::from_utf8(&value).unwrap());
			sp_io::offchain_index::set(&ONCHAIN_TX_KEY, &data.encode());
			Self::deposit_event(Event::SetValue { value, who: _who });
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::info!("OCW ==> Hello World from offchain workers!: {:?}", block_number);
			let value = Self::get_value();
			log::info!(
				"get_value ==> from indexing data [{:?}]",
				sp_std::str::from_utf8(&value).unwrap()
			);
		}
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			log::info!("OCW ==> in on_initialize!");
			Weight::from_parts(0, 0)
		}
		fn on_finalize(_n: BlockNumberFor<T>) {
			log::info!("OCW ==> in on_finalize!");
		}
		fn on_idle(_n: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			log::info!("OCW ==> in on_idle!");
			Weight::from_parts(0, 0)
		}
	}

	impl<T: Config> Pallet<T> {
		fn get_value() -> BoundedVec<u8, ConstU32<4>> {
			match sp_runtime::offchain::storage::StorageValueRef::persistent(ONCHAIN_TX_KEY)
				.get::<IndexingData>()
				.unwrap_or_else(|_| {
					log::info!("OCW ==> Error while fetching data from offchain storage!");
					None
				}) {
				Some(value) => value.0,
				None => Default::default(),
			}
		}
	}
}
