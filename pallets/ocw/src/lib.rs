#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
	SigningTypes,
};
pub use pallet::*;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeDebug,
};

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");
pub mod crypto {
	use super::KEY_TYPE;
	use codec::alloc::string::String;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_system::{
		offchain::{
			AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
			SigningTypes,
		},
		pallet_prelude::*,
	};
	use sp_runtime::{
		offchain::{http, Duration},
		transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
		RuntimeDebug,
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
		#[pallet::call_index(0)]
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

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn unsigned_extrinsic_with_signed_payload(
			origin: OriginFor<T>,
			payload: Payload<T::Public>,
			_signature: T::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			log::info!(
				"OCW ==> in call unsigned_extrinsic_with_signed_payload: {:?}",
				payload.number
			);
			// Return a successful DispatchResultWithPostInfo
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

			let number: u64 = 42;
			let signer = Signer::<T, T::AuthorityId>::any_account();
			if let Some((_, res)) = signer.send_unsigned_transaction(
				// this line is to prepare and return payload
				|acct| Payload { number, public: acct.public.clone() },
				|payload, signature| Call::unsigned_extrinsic_with_signed_payload { payload, signature },
			) {
				match res {
					Ok(()) => {log::info!("OCW ==> unsigned tx with signed payload successfully sent.");}
					Err(()) => {log::error!("OCW ==> sending unsigned tx with signed payload failed.");}
				};
			} else {
				// The case of `None`: no account is available for sending
				log::error!("OCW ==> No local account available");
			}
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

	// 发送未签名交易时需要实现的 trait
	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			const UNSIGNED_TXS_PRIORITY: u64 = 100;
			let valid_tx = |provide| {
				ValidTransaction::with_tag_prefix("my-pallet")
					.priority(UNSIGNED_TXS_PRIORITY) // please define `UNSIGNED_TXS_PRIORITY` before this line
					.and_provides([&provide])
					.longevity(3)
					.propagate(true)
					.build()
			};

			match call {
				Call::unsigned_extrinsic_with_signed_payload { ref payload, ref signature } => {
					if !SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone()) {
						return InvalidTransaction::BadProof.into()
					}
					valid_tx(b"unsigned_extrinsic_with_signed_payload".to_vec())
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}
}
